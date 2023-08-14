#include "newCache.hpp"

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

CacheWriter::CacheWriter(int num_threads) {
    for (int i = 0; i < num_threads; ++i) {
        m_flushers.emplace_back(&CacheWriter::run, this);
    }
}

CacheWriter::CacheWriter::~CacheWriter() {
    flush();
    // stop the threads.
    std::unique_lock lock(m_mtx);
    m_active = false;
    m_run.notify_all();
    lock.unlock();
    for (auto &thr : m_flushers) thr.join();
}

void CacheWriter::run() {
    std::unique_lock lock(m_mtx);
    while (m_active) {
        // do copy jobs:
        if (!m_copy.empty()) {
            auto task = std::move(m_copy.front());
            m_copy.pop_front();
            lock.unlock();

            task();

            lock.lock();
            continue;
        }
        // file flushes:
        if (!m_flushes.empty()) {
            auto task = std::move(m_flushes.front());
            m_flushes.pop_front();
            lock.unlock();

            task();

            lock.lock();
            continue;
        }
        // notify that we are done here.
        m_wait.notify_one();
        // wait for jobs.
        m_run.wait(lock);
    }
    m_wait.notify_one();
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

    auto header = std::make_shared<struct_region<Header>>(file_, 0);
    (*header)->magic = cacheformat::MAGIC;
    (*header)->n = n;
    (*header)->numShapes = hashes.byshape.size();
    (*header)->numPolycubes = hashes.size();

    std::vector<XYZ> keys;
    keys.reserve((*header)->numShapes);
    for (auto &pair : hashes.byshape) keys.push_back(pair.first);
    std::sort(keys.begin(), keys.end());

    auto shapeEntry = std::make_shared<array_region<ShapeEntry>>(file_, header->getEndSeek(), (*header)->numShapes);

    uint64_t offset = shapeEntry->getEndSeek();
    size_t num_cubes = 0;
    int i = 0;
    for (auto &key : keys) {
        auto &se = (*shapeEntry)[i++];
        se.dim0 = key.x();
        se.dim1 = key.y();
        se.dim2 = key.z();
        se.reserved = 0;
        se.offset = offset;
        auto count = hashes.byshape[key].size();
        num_cubes += count;
        se.size = count * XYZ_SIZE * n;
        offset += se.size;
    }

    // put XYZs
    // Serialize large CubeSet(s) in parallel.

    auto xyz = std::make_shared<array_region<XYZ>>(file_, (*shapeEntry)[0].offset, num_cubes * n);
    auto put = xyz->get();

    auto copyrange = [n](CubeSet::iterator itr, CubeSet::iterator end, XYZ *dest) -> void {
        while (itr != end) {
            static_assert(sizeof(XYZ) == XYZ_SIZE);
            assert(itr->size() == n);
            itr->copyout(n, dest);
            dest += n;
            ++itr;
        }
    };

    auto time_start = std::chrono::steady_clock::now();
    for (auto &key : keys) {
        for (auto &subset : hashes.byshape[key].byhash) {
            auto itr = subset.set.begin();

            ptrdiff_t dist = subset.set.size();
            // distribute if range is large enough.
            auto skip = std::max(4096L, std::max(1L, dist / (signed)m_flushers.size()));
            while (dist > skip) {
                auto start = itr;
                auto dest = put;

                auto inc = std::min(dist, skip);
                std::advance(itr, inc);
                put += n * inc;
                dist = std::distance(itr, subset.set.end());

                auto done = 100.0f * (std::distance(xyz->get(), put) / float(num_cubes * n));
                std::printf("writing data %5.2f%% ...  \r", done);
                std::flush(std::cout);

                std::lock_guard lock(m_mtx);
                m_copy.emplace_back(std::bind(copyrange, start, itr, dest));
                m_run.notify_all();
            }
            // copy remainder, if any.
            if (dist) {
                std::lock_guard lock(m_mtx);
                m_copy.emplace_back(std::bind(copyrange, itr, subset.set.end(), put));
                m_run.notify_all();
                put += n * dist;

                auto done = 100.0f * (std::distance(xyz->get(), put) / float(num_cubes * n));
                std::printf("writing data %5.2f%% ...  \r", done);
                std::flush(std::cout);
            }
        }
    }

    // sanity check:
    assert(put == (*xyz).get() + num_cubes * n);

    // sync up.
    std::unique_lock lock(m_mtx);
    while (!m_copy.empty()) {
        m_wait.wait(lock);
    }

    // move the resources into flush job.
    m_flushes.emplace_back(std::bind(
        [](auto &&file, auto &&header, auto &&shapeEntry, auto &&xyz) -> void {
            // flush.
            header->flush();
            shapeEntry->flush();
            xyz->flush();
            // Truncate file to proper size.
            file->truncate(xyz->getEndSeek());
            file->close();
            file.reset();
            xyz.reset();
            shapeEntry.reset();
            header.reset();
        },
        std::move(file_), std::move(header), std::move(shapeEntry), std::move(xyz)));
    m_run.notify_all();

    auto time_end = std::chrono::steady_clock::now();
    auto dt_ms = std::chrono::duration_cast<std::chrono::milliseconds>(time_end - time_start).count();

    std::printf("saved %s, took %.2f s\n\r", path.c_str(), dt_ms / 1000.f);
}

void CacheWriter::flush() {
    std::unique_lock lock(m_mtx);
    while (!m_flushes.empty()) {
        m_wait.wait(lock);
    }
}
