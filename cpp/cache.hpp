#pragma once
#include <unordered_set>
#include <fstream>
#include "structs.hpp"

unordered_set<Cube> load(string path)
{
    auto ifs = ifstream(path, ios::binary);
    if (!ifs.is_open())
        return {};
    uint8_t cubelen = 0;
    uint filelen = ifs.tellg();
    ifs.seekg(0, ios::end);
    filelen = (uint)ifs.tellg() - filelen;
    ifs.seekg(0, ios::beg);
    ifs.read((char *)&cubelen, 1);
    printf("loading cache file \"%s\" (%u bytes) with N = %d\n\r", path.c_str(), filelen, cubelen);

    auto cubeSize = 4 * (int)cubelen;
    auto numCubes = (filelen - 1) / cubeSize;
    if (numCubes * cubeSize + 1 != filelen)
    {
        printf("error reading file, size does not match");
        return {};
    }
    printf("  num polycubes loading: %d\n\r", numCubes);
    unordered_set<Cube> cubes;
    for (int i = 0; i < numCubes; ++i)
    {
        Cube next;
        next.sparse.resize(cubelen);
        for (int k = 0; k < cubelen; ++k)
        {
            ifs.read((char *)&next.sparse[k].joined, 4);
        }
        cubes.insert(next);
    }
    printf("  loaded %lu cubes\n\r", cubes.size());
    return cubes;
}

void save(string path, unordered_set<Cube> &cubes)
{
    if (cubes.size() == 0)
        return;
    ofstream ofs(path, ios::binary);
    ofs << (uint8_t)cubes.begin()->sparse.size();
    for (const auto &c : cubes)
    {
        for (const auto &p : c.sparse)
        {
            ofs.write((const char *)&p.joined, sizeof(p.joined));
        }
    }
}