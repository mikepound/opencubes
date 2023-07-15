#pragma once
#ifndef OPENCUBES_CACHE_HPP
#define OPENCUBES_CACHE_HPP
#include <fstream>
#include <limits>
#include <string>
#include <unordered_set>

#include "structs.hpp"

Hashy load(std::string path) {
    Hashy cubes;
    auto ifs = std::ifstream(path, std::ios::binary);
    if (!ifs.is_open()) return cubes;
    uint8_t cubelen = 0;
    // read big filesize by reading file from:
    // https://stackoverflow.com/questions/2409504/using-c-filestreams-fstream-how-can-you-determine-the-size-of-a-file
    ifs.ignore(std::numeric_limits<std::streamsize>::max());
    uint64_t filelen = ifs.gcount();
    ifs.clear();  //  Since ignore will have set eof.
    ifs.seekg(0, std::ios_base::beg);

    ifs.read((char *)&cubelen, 1);
    std::printf("loading cache file \"%s\" (%lu bytes) with N = %d\n\r", path.c_str(), filelen, cubelen);

    auto cubeSize = 4 * (uint)cubelen;
    auto numCubes = (filelen - 1U) / cubeSize;
    if (numCubes * cubeSize + 1U != filelen) {
        std::printf("error reading file, size does not match\n\r");
        std::printf("  cubeSize = %u bytes, numCubes = %lu\n\r", cubeSize, numCubes);
        return cubes;
    }
    std::printf("  num polycubes loading: %ld\n\r", numCubes);
    cubes.init(cubelen);
    for (size_t i = 0; i < numCubes; ++i) {
        Cube next;
        next.sparse.resize(cubelen);
        XYZ shape;
        for (int k = 0; k < cubelen; ++k) {
            uint32_t tmp;
            ifs.read((char *)&tmp, 4);
            next.sparse[k][0] = (tmp >> 16) & 0xff;
            next.sparse[k][1] = (tmp >> 8) & 0xff;
            next.sparse[k][2] = (tmp)&0xff;
            if (next.sparse[k].x() > shape.x()) shape.x() = next.sparse[k].x();
            if (next.sparse[k].y() > shape.y()) shape.y() = next.sparse[k].y();
            if (next.sparse[k].z() > shape.z()) shape.z() = next.sparse[k].z();
        }
        cubes.insert(next, shape);
    }
    std::printf("  loaded %lu cubes\n\r", cubes.size());
    return cubes;
}

void save(std::string path, Hashy &cubes, uint8_t n) {
    if (cubes.size() == 0) return;
    std::ofstream ofs(path, std::ios::binary);
    ofs << n;
    for (const auto &s : cubes.byshape)
        for (const auto &c : s.second.set) {
            for (const auto &p : c) {
                uint32_t tmp = p;
                ofs.write((const char *)&tmp, sizeof(tmp));
            }
        }
    std::printf("saved %s\n\r", path.c_str());
}

#endif
