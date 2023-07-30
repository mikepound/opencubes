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
    header_ = std::make_unique<const mapped::struct_region<Header>>(file_, 0);
    header = header_->get();

    if (header->magic != MAGIC) {
        std::printf("error opening file: file not recognized\n");
        return 1;
    }

    // map the ShapeEntry array:
    shapes_ = std::make_unique<const mapped::array_region<ShapeEntry>>(file_, header_->getEndSeek(), (*header_)->numShapes);
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
    auto index = offset / XYZ_SIZE;
    auto num_xyz = shapes[i].size / XYZ_SIZE;
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
