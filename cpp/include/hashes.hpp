#pragma once
#ifndef OPENCUBES_HASHES_HPP
#define OPENCUBES_HASHES_HPP
#include <array>
#include <cstdio>
#include <map>
#include <shared_mutex>
#include <unordered_set>
#include <vector>

#include "cube.hpp"
#include "utils.hpp"

struct HashCube {
    size_t operator()(const Cube &cube) const {
        // https://stackoverflow.com/questions/20511347/a-good-hash-function-for-a-vector/72073933#72073933
        std::size_t seed = cube.size();
        for (auto &p : cube) {
            auto x = HashXYZ()(p);
            seed ^= x + 0x9e3779b9 + (seed << 6) + (seed >> 2);
        }
        return seed;
    }
};

using CubeSet = std::unordered_set<Cube, HashCube, std::equal_to<Cube>>;

struct Hashy {
    struct Subsubhashy {
        CubeSet set;
        std::shared_mutex set_mutex;

        template <typename CubeT>
        void insert(CubeT &&c) {
            std::lock_guard lock(set_mutex);
            set.emplace(std::forward<CubeT>(c));
        }

        bool contains(const Cube &c) {
            std::shared_lock lock(set_mutex);
            return set.count(c);
        }

        auto size() {
            std::shared_lock lock(set_mutex);
            return set.size();
        }
    };
    template <int NUM>
    struct Subhashy {
        std::array<Subsubhashy, NUM> byhash;

        template <typename CubeT>
        void insert(CubeT &&c) {
            HashCube hash;
            auto idx = hash(c) % NUM;
            auto &set = byhash[idx];
            if (!set.contains(c)) set.insert(std::forward<CubeT>(c));
            // printf("new size %ld\n\r", byshape[shape].size());
        }

        auto size() {
            size_t sum = 0;
            for (auto &set : byhash) {
                auto part = set.size();
                sum += part;
            }
            return sum;
        }
    };

    std::map<XYZ, Subhashy<8>> byshape;
    void init(int n) {
        // create all subhashy which will be needed for N
        for (int x = 0; x < n; ++x)
            for (int y = x; y < (n - x); ++y)
                for (int z = y; z < (n - x - y); ++z) {
                    if ((x + 1) * (y + 1) * (z + 1) < n)  // not enough space for n cubes
                        continue;
                    byshape[XYZ(x, y, z)].size();
                }
        std::printf("%ld sets by shape for N=%d\n\r", byshape.size(), n);
    }

    template <typename CubeT>
    void insert(CubeT &&c, XYZ shape) {
        auto &set = byshape[shape];
        set.insert(std::forward<CubeT>(c));
    }

    auto size() {
        size_t sum = 0;
        DEBUG_PRINTF("%ld maps by shape\n\r", byshape.size());
        for (auto &set : byshape) {
            auto part = set.second.size();
            DEBUG_PRINTF("bucket [%2d %2d %2d]: %ld\n", set.first.x(), set.first.y(), set.first.z(), part);
            sum += part;
        }
        return sum;
    }
};
#endif
