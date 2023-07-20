#pragma once
#ifndef OPENCUBES_NEWCACHE_HPP
#define OPENCUBES_NEWCACHE_HPP
#include <cstring>
#include <string>

#include "cube.hpp"
#include "hashes.hpp"

struct CubeView {
    uint32_t n;
    const XYZ* sparse;
    void print() const {
        for (uint32_t i = 0; i < n; ++i) {
            printf("(%2d %2d %2d) ", sparse[i].x(), sparse[i].y(), sparse[i].z());
        }
        printf("\n");
    }
    operator Cube() const {
        Cube ret(n);
        memcpy(ret.data(), sparse, n * sizeof(XYZ));
        return ret;
    }
};
class Workset;
struct CubeIterator {
    using iterator_category = std::forward_iterator_tag;
    using difference_type = std::ptrdiff_t;
    using value_type = CubeView;
    using pointer = CubeView*;    // or also value_type*
    using reference = CubeView&;  // or also value_type&

   public:
    // constructor
    CubeIterator(uint32_t n, uint8_t* ptr) : n(n), m_ptr(ptr) {}

    // operators
    const value_type operator*() const {
        value_type ret{n, (XYZ*)m_ptr};
        return ret;
    }
    // pointer operator->() { return (pointer)m_ptr; }

    // Prefix increment
    CubeIterator& operator++() {
        m_ptr += 3 * n;
        return *this;
    }
    CubeIterator& operator+=(int incr) {
        m_ptr += 3 * n * incr;
        return *this;
    }

    // Postfix increment
    CubeIterator operator++(int) {
        CubeIterator tmp = *this;
        ++(*this);
        return tmp;
    }

    friend bool operator==(const CubeIterator& a, const CubeIterator& b) { return a.m_ptr == b.m_ptr; };
    friend bool operator<(const CubeIterator& a, const CubeIterator& b) { return a.m_ptr < b.m_ptr; };
    friend bool operator>(const CubeIterator& a, const CubeIterator& b) { return a.m_ptr > b.m_ptr; };
    friend bool operator!=(const CubeIterator& a, const CubeIterator& b) { return a.m_ptr != b.m_ptr; };
    friend class Workset;

   private:
    uint32_t n;
    uint8_t* m_ptr;
};
struct ShapeRange {
    CubeIterator begin() { return b; }
    CubeIterator end() { return e; }

    CubeIterator b, e;
    uint64_t size;
    XYZ shape;
};

struct ICache {
    virtual ShapeRange getCubesByShape(uint32_t i) = 0;
    virtual uint32_t numShapes() = 0;
    virtual size_t size() = 0;
};

class CacheReader : public ICache {
   public:
    // constructor
    explicit CacheReader();
    // destuctor
    ~CacheReader();

    // methods
    void printHeader();
    int printShapes();
    int loadFile(const std::string path);

    // vars
    char* data;
    uint8_t* filePointer;
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

    CubeIterator begin() { return CubeIterator(header->n, filePointer + shapes[0].offset); }
    CubeIterator end() { return CubeIterator(header->n, filePointer + shapes[0].offset + header->numPolycubes * header->n * 3); }

    virtual ShapeRange getCubesByShape(uint32_t i) override {
        if (i >= header->numShapes) return {CubeIterator(header->n, 0), CubeIterator(header->n, 0), 0, XYZ(0, 0, 0)};
        return {CubeIterator(header->n, filePointer + shapes[i].offset), CubeIterator(header->n, filePointer + shapes[i].offset + shapes[i].size),
                shapes[i].size / (header->n * sizeof(XYZ)), XYZ(shapes[i].dim0, shapes[i].dim1, shapes[i].dim2)};
    }
    virtual size_t size() override { return header->numPolycubes; };
    virtual uint32_t numShapes() override { return header->numShapes; };
    operator bool() { return fileLoaded_; }

   private:
    // private vars
    std::string path_;
    int fileDescriptor_;
    uint64_t fileSize_;
    bool fileLoaded_;
    Header dummyHeader;
    Header* header;
    ShapeEntry* shapes;
};

struct FlatCache : public ICache {
    std::vector<XYZ> allXYZs;
    std::vector<ShapeRange> shapes;
    uint8_t n = 0;
    FlatCache() {}
    FlatCache(Hashy& hashes, uint8_t n) : n(n) {
        allXYZs.reserve(hashes.size() * n);
        shapes.reserve(hashes.byshape.size());
        // std::printf("Flatcache %d %p %p\n", n, (void*)allXYZs.data(), (void*)shapes.data());
        for (auto& [shape, set] : hashes.byshape) {
            auto begin = (uint8_t*)&*allXYZs.end();
            for (auto& subset : set.byhash) {
                for (auto& cube : subset.set)
                    // allXYZs.emplace_back(allXYZs.end(), subset.set.begin(), subset.set.end());
                    std::copy(cube.begin(), cube.end(), std::back_inserter(allXYZs));
            }
            auto end = (uint8_t*)&*allXYZs.end();
            // std::printf("  SR %p %p\n", (void*)begin, (void*)end);
            ShapeRange sr{CubeIterator(n, begin), CubeIterator(n, end), set.size(), shape};
            shapes.emplace_back(sr);
        }
    }
    virtual ShapeRange getCubesByShape(uint32_t i) override {
        if (i >= shapes.size()) return {CubeIterator(0, 0), CubeIterator(0, 0), 0, XYZ(0, 0, 0)};
        return shapes[i];
    };
    virtual uint32_t numShapes() override { return shapes.size(); };
    virtual size_t size() override { return allXYZs.size() / n / sizeof(XYZ); }
};

#endif