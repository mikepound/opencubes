#pragma once
#ifndef OPENCUBES_STRUCTS_HPP
#define OPENCUBES_STRUCTS_HPP
#include <cstdio>
#include <map>
#include <shared_mutex>
#include <unordered_set>
#include <vector>

// #define DBG 1
struct XYZ {
    int8_t data[3];
    explicit XYZ(int8_t a = 0, int8_t b = 0, int8_t c = 0) : data{a, b, c} {}
    constexpr bool operator==(const XYZ &b) const { return (uint32_t) * this == (uint32_t)b; }
    constexpr bool operator<(const XYZ &b) const { return (uint32_t) * this < (uint32_t)b; }
    constexpr operator uint32_t() const { return ((uint8_t)data[0] << 16) | ((uint8_t)data[1] << 8) | ((uint8_t)data[2]); }

    constexpr int8_t &x() { return data[0]; }
    constexpr int8_t &y() { return data[1]; }
    constexpr int8_t &z() { return data[2]; }
    constexpr int8_t x() const { return data[0]; }
    constexpr int8_t y() const { return data[1]; }
    constexpr int8_t z() const { return data[2]; }
    constexpr int8_t &operator[](int offset) { return data[offset]; }
    constexpr int8_t operator[](int offset) const { return data[offset]; }
};

struct HashXYZ {
    size_t operator()(const XYZ &p) const { return (uint32_t)p; }
};

using XYZSet = std::unordered_set<XYZ, HashXYZ, std::equal_to<XYZ>>;

struct Cube {
    std::vector<XYZ> sparse;
    /**
     * Define subset of vector operations for Cube
     * This simplifies the code everywhere else.
     */
    std::vector<XYZ>::iterator begin() { return sparse.begin(); }

    std::vector<XYZ>::iterator end() { return sparse.end(); }

    std::vector<XYZ>::const_iterator begin() const { return sparse.begin(); }

    std::vector<XYZ>::const_iterator end() const { return sparse.end(); }

    size_t size() const { return sparse.size(); }

    void reserve(size_t N) { sparse.reserve(N); }

    template <typename T>
    T &emplace_back(T &&p) {
        return sparse.emplace_back(std::forward<T>(p));
    }

    bool operator==(const Cube &rhs) const { return this->sparse == rhs.sparse; }

    bool operator<(const Cube &b) const {
        if (size() != b.size()) return size() < b.size();
        for (size_t i = 0; i < size(); ++i) {
            if (sparse[i] < b.sparse[i])
                return true;
            else if (sparse[i] > b.sparse[i])
                return false;
        }
        return false;
    }

    void print() const {
        for (auto &p : sparse) std::printf("  (%2d %2d %2d)\n\r", p.x(), p.y(), p.z());
    }
};

struct HashCube {
    size_t operator()(const Cube &cube) const {
        // https://stackoverflow.com/questions/20511347/a-good-hash-function-for-a-vector/72073933#72073933
        std::size_t seed = cube.size();
        for (auto &p : cube) {
            auto x = HashXYZ()(p);
            // x = ((x >> 16) ^ x) * 0x45d9f3b;
            // x = ((x >> 16) ^ x) * 0x45d9f3b;
            // x = (x >> 16) ^ x;
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

        template <typename CubeT>
        bool contains(CubeT &&c) {
            std::shared_lock lock(set_mutex);
            return set.count(std::forward<CubeT>(c));
        }

        auto size() {
            std::shared_lock lock(set_mutex);
            return set.size();
        }
    };
    template <uint NUM>
    struct Subhashy {
        std::array<Subsubhashy, NUM> byhash;

        template <typename CubeT>
        void insert(CubeT &&c) {
            HashCube hash;
            auto idx = hash(c) % NUM;
            auto &set = byhash[idx];
            if (!set.contains(std::forward<CubeT>(c))) set.insert(std::forward<CubeT>(c));
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
#ifndef NDEBUG
        // printf("insert into shape %d %d %d\n", shape.x(), shape.y(), shape.z());
        // c.print();
        if (byshape.find(shape) == byshape.end()) {
            printf("ERROR! shape %d %d %d should already be in map!\n\r", shape.x(), shape.y(), shape.z());
            exit(-1);
        }
#endif
        auto &set = byshape[shape];
        set.insert(std::forward<CubeT>(c));
    }

    auto size() {
        size_t sum = 0;
#ifdef DBG
        std::printf("%ld maps by shape\n\r", byshape.size());
#endif
        for (auto &set : byshape) {
            auto part = set.second.size();
#ifdef DBG
            std::printf("bucket [%2d %2d %2d]: %ld\n", set.first.x(), set.first.y(), set.first.z(), part);
#endif
            sum += part;
        }
        return sum;
    }
};
#endif
