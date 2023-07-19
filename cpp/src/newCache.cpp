#include "../include/newCache.hpp"

#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

#include <iostream>

CacheReader::CacheReader(const std::string& path)
    : path_(path), fileDescriptor_(0), fileSize_(0), fileLoaded_(false), dummyHeader{0, 0, 0, 0}, header(&dummyHeader), shapes(0) {
    if (loadFile(path) != 0) {
        std::cerr << "failed to load data from \"" << path << "\"" << std::endl;
    }
}
void CacheReader::printHeader() {
    if (fileLoaded_) {
        printf("magic: %x ", header->magic);
        printf("n: %d ", header->n);
        printf("numShapes: %d ", header->numShapes);
        printf("numPolycubes: %ld\n", header->numPolycubes);
    } else {
        printf("no file loaded!\n");
    }
}

int CacheReader::printShapes(void) {
    if (fileLoaded_) {
        for (uint64_t i = 0; i < header->numShapes; i++) {
            printf("%d\t%d\t%d\n", shapes[i].dim0, shapes[i].dim1, shapes[i].dim2);
        }
        return 1;
    }
    return 0;
}

int CacheReader::loadFile(std::string path) {
    path_ = path;
    fileDescriptor_ = open(path.c_str(), O_RDONLY);

    if (fileDescriptor_ == -1) {
        std::cerr << "error opening file" << std::endl;
        return 1;
    }

    // get filesize
    fileSize_ = lseek(fileDescriptor_, 0, SEEK_END);
    lseek(fileDescriptor_, 0, SEEK_SET);

    // memory map file
    filePointer = (uint8_t*)mmap(NULL, fileSize_, PROT_READ, MAP_PRIVATE, fileDescriptor_, 0);
    if (filePointer == MAP_FAILED) {
        // error handling
        std::cerr << "errorm mapping file memory" << std::endl;
        close(fileDescriptor_);
        return 2;
    }

    header = (Header*)(filePointer);
    shapes = (ShapeEntry*)(filePointer + sizeof(Header));
    data = (char*)(filePointer + sizeof(Header) + header->numShapes * sizeof(ShapeEntry));

    fileLoaded_ = true;

    return 0;
}

CacheReader::~CacheReader() {
    // unmap file from memory
    if (munmap(filePointer, fileSize_) == -1) {
        // error handling
        std::cerr << "error unmapping file" << std::endl;
    }

    // close file descriptor
    close(fileDescriptor_);
    fileLoaded_ = false;
}
/*
int main(int argc, char** argv) {
    CacheReader cr(argv[1]);
    printf("----------\n");
    cr.printShapes();
    printf("---------\n");
    printf("%d\n", cr.header->numShapes);
    return 0;
}
*/