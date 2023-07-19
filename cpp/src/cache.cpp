#include "cache.hpp"

#include <algorithm>
#include <fstream>
#include <limits>
#include <string>
#include <unordered_set>

#include "utils.hpp"

/*
====================
cache file header
====================

uint32_t magic = "PCUB"
uint32_t n = cache file for n cubes in a polycube
uint32_t numShapes = number of different shapes in cachefile
-------

====================
shapetable:
====================
shapeEntry {
    uint8_t dim0 // offset by -1
    uint8_t dim1 // offset by -1
    uint8_t dim2 // offset by -1
    uint8_t reserved
    uint64_t offset in file
}
shapeEntry[numShapes]


====================
XYZ data
====================

*/

void Cache::save(std::string path, Hashy &hashes, uint8_t n) {
    if (hashes.size() == 0) return;
    std::ofstream ofs(path, std::ios::binary);
    Header header;
    header.magic = MAGIC;
    header.n = n;
    header.numShapes = hashes.byshape.size();
    header.numPolycubes = hashes.size();
    ofs.write((const char *)&header, sizeof(header));

    std::vector<XYZ> keys;
    keys.reserve(header.numShapes);
    for (auto &pair : hashes.byshape) keys.push_back(pair.first);
    std::sort(keys.begin(), keys.end());
    uint64_t offset = sizeof(Header) + header.numShapes * sizeof(ShapeEntry);
    for (auto &key : keys) {
        ShapeEntry se;
        se.dim0 = key.x();
        se.dim1 = key.y();
        se.dim2 = key.z();
        se.reserved = 0;
        se.offset = offset;
        se.size = hashes.byshape[key].size() * XYZ_SIZE * n;
        offset += se.size;
        ofs.write((const char *)&se, sizeof(ShapeEntry));
    }
    // put XYZs
    for (auto &key : keys) {
        for (auto &subset : hashes.byshape[key].byhash)
            for (const auto &c : subset.set) {
                if constexpr (sizeof(XYZ) == XYZ_SIZE) {
                    ofs.write((const char *)c.data(), sizeof(XYZ) * c.size());
                } else {
                    for (const auto &p : c) {
                        ofs.write((const char *)p.data, XYZ_SIZE);
                    }
                }
            }
    }

    std::printf("saved %s\n\r", path.c_str());
}

Hashy Cache::load(std::string path, uint32_t extractShape) {
    Hashy cubes;
    auto ifs = std::ifstream(path, std::ios::binary);
    if (!ifs.is_open()) return cubes;
    Header header;
    if (!ifs.read((char *)&header, sizeof(header))) {
        return cubes;
    }
    // check magic
    if (header.magic != MAGIC) {
        return cubes;
    }
#ifdef CACHE_LOAD_HEADER_ONLY
    std::printf("loading cache file \"%s\" for N = %u", path.c_str(), header.n);
    std::printf(", %u shapes, %lu XYZs\n\r", header.numShapes, header.numPolycubes);
#endif
    auto cubeSize = XYZ_SIZE * header.n;
    DEBUG_PRINTF("cubeSize: %u\n\r", cubeSize);

    for (uint32_t i = 0; i < header.numShapes; ++i) {
        ShapeEntry shapeEntry;
        if (!ifs.read((char *)&shapeEntry, sizeof(shapeEntry))) {
            std::printf("ERROR reading ShapeEntry %u\n\r", i);
            exit(-1);
        }
        if (ALL_SHAPES != extractShape && i != extractShape) continue;
#ifdef CACHE_PRINT_SHAPEENTRIES
        std::printf("ShapeEntry %3u: [%2d %2d %2d] offset: 0x%08lx size: 0x%08lx (%ld polycubes)\n\r", i, shapeEntry.dim0, shapeEntry.dim1, shapeEntry.dim2,
                    shapeEntry.offset, shapeEntry.size, shapeEntry.size / cubeSize);
#endif
        if (shapeEntry.size % cubeSize != 0) {
            std::printf("ERROR shape block is not divisible by cubeSize!\n\r");
            exit(-1);
        }
#ifndef CACHE_LOAD_HEADER_ONLY
        // remember pos in file
        auto pos = ifs.tellg();

        // read XYZ contents
        ifs.seekg(shapeEntry.offset);
        const uint32_t CHUNK_SIZE = 512 * XYZ_SIZE;
        uint8_t buf[CHUNK_SIZE] = {0};
        uint64_t buf_offset = 0;
        uint32_t numCubes = shapeEntry.size / cubeSize;
        XYZ shape(shapeEntry.dim0, shapeEntry.dim1, shapeEntry.dim2);
        uint64_t readsize = shapeEntry.size - buf_offset;
        if (readsize > CHUNK_SIZE) readsize = CHUNK_SIZE;
        if (!ifs.read((char *)&buf, readsize)) {
            std::printf("ERROR reading XYZs for Shape %u\n\r", i);
            exit(-1);
        }
        for (uint32_t j = 0; j < numCubes; ++j) {
            Cube next(header.n);
            for (uint32_t k = 0; k < header.n; ++k) {
                // check if buf contains next XYZ
                uint64_t curr_offset = j * cubeSize + k * XYZ_SIZE;
                if (curr_offset >= buf_offset + CHUNK_SIZE) {
                    // std::printf("reload buffer\n\r");
                    buf_offset += CHUNK_SIZE;
                    readsize = shapeEntry.size - buf_offset;
                    if (readsize > CHUNK_SIZE) readsize = CHUNK_SIZE;
                    if (!ifs.read((char *)&buf, readsize)) {
                        std::printf("ERROR reading XYZs for Shape %u\n\r", i);
                        exit(-1);
                    }
                }

                next.data()[k].data[0] = buf[curr_offset - buf_offset + 0];
                next.data()[k].data[1] = buf[curr_offset - buf_offset + 1];
                next.data()[k].data[2] = buf[curr_offset - buf_offset + 2];
            }
            cubes.insert(next, shape);
        }

        // restore pos
        ifs.seekg(pos);
#endif
    }
    std::printf("  loaded %lu cubes\n\r", cubes.size());
    return cubes;
}
