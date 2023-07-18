#pragma once
#ifndef OPENCUBES_CACHE_HPP
#define OPENCUBES_CACHE_HPP
#include <string>

#include "hashes.hpp"
#include "utils.hpp"

struct Cache {
    static constexpr uint32_t MAGIC = 0x42554350;
    static constexpr uint32_t XYZ_SIZE = 3;
    static constexpr uint32_t ALL_SHAPES = -1;
    struct Header {
        uint32_t magic = MAGIC;  // shoud be "PCUB" = 0x42554350
        uint32_t n;              // we will never need 32bit but it is nicely aligned
        uint32_t numShapes;      // defines length of the shapeTable
        uint64_t numPolycubes;   // total number of polycubes
    };
    struct ShapeEntry {
        uint8_t dim0;      // offset by -1
        uint8_t dim1;      // offset by -1
        uint8_t dim2;      // offset by -1
        uint8_t reserved;  // for alignment
        uint64_t offset;   // from beginning of file
        uint64_t size;     // in bytes should be multiple of XYZ_SIZE
    };

    static void save(std::string path, Hashy &hashes, uint8_t n);
    static Hashy load(std::string path, uint32_t extractShape = ALL_SHAPES);
};

#endif
