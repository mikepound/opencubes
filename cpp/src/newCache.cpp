#include "../include/newCache.hpp"

#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

#include <iostream>

CacheReader::CacheReader() : path_(""), fileLoaded_(false), dummyHeader{0, 0, 0, 0}, header(&dummyHeader), shapes(nullptr) {}

void CacheReader::printHeader() {
    if (fileLoaded_) {
        std::printf("magic: %x ", header->magic);
        std::printf("n: %d ", header->n);
        std::printf("numShapes: %d ", header->numShapes);
        std::printf("numPolycubes: %ld\n", header->numPolycubes);
    } else {
        std::printf("no file loaded!\n");
    }
}

int CacheReader::printShapes(void) {
    if (fileLoaded_) {
        for (uint64_t i = 0; i < header->numShapes; i++) {
            std::printf("%d\t%d\t%d\n", shapes[i].dim0, shapes[i].dim1, shapes[i].dim2);
        }
        return 1;
    }
    return 0;
}

int CacheReader::loadFile(const std::string path) {
    unload();
    path_ = path;

    // open read-only backing file:
    file_ = std::make_shared<mapped::file>();
    if (file_->open(path.c_str())) {
        std::printf("error opening file\n");
        return 1;
    }

    // map the header struct
    header_ = std::make_unique<const mapped::struct_region<cacheformat::Header>>(file_, 0);
    header = header_->get();

    if (header->magic != cacheformat::MAGIC) {
        std::printf("error opening file: file not recognized\n");
        return 1;
    }

    // map the ShapeEntry array:
    shapes_ = std::make_unique<const mapped::array_region<cacheformat::ShapeEntry>>(file_, header_->getEndSeek(), (*header_)->numShapes);
    shapes = shapes_->get();

    size_t datasize = 0;
    for (unsigned int i = 0; i < header->numShapes; ++i) {
        datasize += shapes[i].size;
    }

    // map rest of the file as XYZ data:
    if (file_->size() != shapes_->getEndSeek() + datasize) {
        std::printf("warn: file size does not match expected value\n");
    }
    xyz_ = std::make_unique<const mapped::array_region<XYZ>>(file_, shapes_->getEndSeek(), datasize);

    fileLoaded_ = true;

    return 0;
}

ShapeRange CacheReader::getCubesByShape(uint32_t i) {
    if (i >= header->numShapes) {
        return ShapeRange{nullptr, nullptr, 0, XYZ(0, 0, 0)};
    }
    if (shapes[i].size <= 0) {
        return ShapeRange{nullptr, nullptr, header->n, XYZ(shapes[i].dim0, shapes[i].dim1, shapes[i].dim2)};
    }
    // get section start
    // note: shapes[i].offset may have bogus offset
    // if any earlier shape table entry was empty before i
    // so we ignore the offset here.
    size_t offset = 0;
    for (unsigned int k = 0; k < i; ++k) {
        offset += shapes[k].size;
    }
    auto index = offset / cacheformat::XYZ_SIZE;
    auto num_xyz = shapes[i].size / cacheformat::XYZ_SIZE;
    // pointers to Cube data:
    auto start = xyz_->get() + index;
    auto end = xyz_->get() + index + num_xyz;
    return ShapeRange{start, end, header->n, XYZ(shapes[i].dim0, shapes[i].dim1, shapes[i].dim2)};
}

void CacheReader::unload() {
    // unload file from memory
    if (fileLoaded_) {
        xyz_.reset();
        shapes_.reset();
        header_.reset();
        file_.reset();
        fileLoaded_ = false;
    }
    header = &dummyHeader;
    shapes = nullptr;
}

CacheReader::~CacheReader() { unload(); }

CacheWriter::CacheWriter::~CacheWriter()
{
    flush();
}


void CacheWriter::save(std::string path, Hashy &hashes, uint8_t n) {
    if (hashes.size() == 0) return;

    using namespace mapped;
    using namespace cacheformat;

    auto file_ = std::make_shared<file>();
    if (file_->openrw(path.c_str(), 0)) {
        std::printf("error opening file\n");
        return;
    }

    auto header = std::make_unique<struct_region<Header>>(file_, 0);
    (*header)->magic = cacheformat::MAGIC;
    (*header)->n = n;
    (*header)->numShapes = hashes.byshape.size();
    (*header)->numPolycubes = hashes.size();

    std::vector<XYZ> keys;
    keys.reserve((*header)->numShapes);
    for (auto &pair : hashes.byshape) keys.push_back(pair.first);
    std::sort(keys.begin(), keys.end());

    auto shapeEntry = std::make_unique<array_region<ShapeEntry>>(file_, header->getEndSeek(), (*header)->numShapes);

    uint64_t offset = shapeEntry->getEndSeek();
    size_t num_cubes = 0;
    int i = 0;
    for (auto &key : keys) {
        auto& se = (*shapeEntry)[i++];
        se.dim0 = key.x();
        se.dim1 = key.y();
        se.dim2 = key.z();
        se.reserved = 0;
        se.offset = offset;
        auto count = hashes.byshape[key].size() ;
        num_cubes += count;
        se.size = count * XYZ_SIZE * n;
        offset += se.size;
    }

    // put XYZs
    // do this in parallel?
    // it takes an long while to write out the file.
    // note: we are at peak memory use in this function.

    auto xyz = std::make_unique<array_region<XYZ>>(file_, (*shapeEntry)[0].offset, num_cubes * n);
    auto put = xyz->get();

    for (auto &key : keys) {
        for (auto &subset : hashes.byshape[key].byhash) {
            auto itr = subset.set.begin();
            while(itr != subset.set.end()) {
                static_assert(sizeof(XYZ) == XYZ_SIZE);
                assert(itr->size() == n);
                itr->copyout(n, put);
                put += n;
                ++itr;
            }
        }
    }
    // move the resources into lambda and async launch it.
    // the file is finalized in background.
    m_flushes.emplace_back(std::async(std::launch::async, [
        file = std::move(file_),
        header = std::move(header),
        shapeEntry = std::move(shapeEntry),
        xyz = std::move(xyz)]() mutable {
            // flush.
            header->flush();
            shapeEntry->flush();
            xyz->flush();
            // Truncate file to proper size.
            file->truncate(xyz->getEndSeek());
            file->close();
            xyz.reset();
            shapeEntry.reset();
            header.reset();
            file.reset();
    }));

    // cleanup completed flushes. (don't wait)
    auto rm = std::remove_if(m_flushes.begin(), m_flushes.end(), [](auto& fut) {
        if(fut.wait_for(std::chrono::seconds(0)) == std::future_status::ready) {
            fut.get();
            return true;
        }
        return false;
    });
    m_flushes.erase(rm, m_flushes.end());

    std::printf("saved %s, %d unfinished.\n\r", path.c_str(), (int)m_flushes.size());
}

void CacheWriter::flush()
{
    for(auto& fut : m_flushes) {
        fut.get();
    }
    m_flushes.clear();
}

