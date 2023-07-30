#include "../include/newCache.hpp"

#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

#include <iostream>

CacheReader::CacheReader()
    : filePointer(nullptr), path_(""), fileDescriptor_(-1), fileSize_(0), fileLoaded_(false), dummyHeader{0, 0, 0, 0}, header(&dummyHeader), shapes(nullptr) {}

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
    fileDescriptor_ = open(path.c_str(), O_RDONLY);

    if (fileDescriptor_ == -1) {
        std::printf("error opening file\n");
        return 1;
    }

    // get filesize
    fileSize_ = lseek(fileDescriptor_, 0, SEEK_END);
    lseek(fileDescriptor_, 0, SEEK_SET);

    // memory map file
    filePointer = (uint8_t*)mmap(NULL, fileSize_, PROT_READ, MAP_SHARED, fileDescriptor_, 0);
    if (filePointer == MAP_FAILED) {
        // error handling
        std::printf("errorm mapping file memory");
        close(fileDescriptor_);
        return 2;
    }

    header = (Header*)(filePointer);
    shapes = (ShapeEntry*)(filePointer + sizeof(Header));

    fileLoaded_ = true;

    return 0;
}

ShapeRange CacheReader::getCubesByShape(uint32_t i) {
    if (i >= header->numShapes) {
        return ShapeRange{nullptr, nullptr, 0, XYZ(0, 0, 0)};
    }
    if(shapes[i].size <= 0) {
        return ShapeRange(nullptr, nullptr, header->n, XYZ(shapes[i].dim0, shapes[i].dim1, shapes[i].dim2));
    }
    auto start = reinterpret_cast<const XYZ*>(filePointer + shapes[i].offset);
    auto end = reinterpret_cast<const XYZ*>(filePointer + shapes[i].offset + shapes[i].size);
    return ShapeRange(start, end, header->n, XYZ(shapes[i].dim0, shapes[i].dim1, shapes[i].dim2));
}

void CacheReader::unload() {
    // unmap file from memory
    if (fileLoaded_) {
        if (munmap((void*)filePointer, fileSize_) == -1) {
            // error handling
            std::printf("error unmapping file\n");
        }

        // close file descriptor
        close(fileDescriptor_);
        fileLoaded_ = false;
    }
    fileDescriptor_ = -1;
    filePointer = nullptr;
    header = &dummyHeader;
    shapes = nullptr;
}

CacheReader::~CacheReader() { unload(); }
