#pragma once
#ifndef OPENCUBES_NEWCACHE_HPP
#define OPENCUBES_NEWCACHE_HPP
#include <cstring>
#include <string>

#include "cube.hpp"

struct CubeView {
    uint32_t n;
    const XYZ* sparse;
    void print() const {
        for (uint32_t i = 0; i < n; ++i) {
            std::printf("(%2d %2d %2d) ", sparse[i].x(), sparse[i].y(), sparse[i].z());
        }
        std::printf("\n");
    }
    operator Cube() const {
        Cube ret(n);
        std::copy_n(sparse, n, ret.begin());
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
    CubeIterator(uint32_t _n, uint8_t* ptr) : n(_n), m_ptr(ptr) {}

    CubeIterator() : n(0), m_ptr(nullptr) {}

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
class CacheReader {
   public:
    // constructor
    explicit CacheReader(const std::string& path);
    // destuctor
    ~CacheReader();

    // methods
    void printHeader();
    int printShapes();
    int loadFile(std::string path);

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

    ShapeRange getCubesByShape(uint32_t i) {
        ShapeRange range;
        if (i >= header->numShapes) {
            range.b = CubeIterator(header->n, 0);
            range.e = CubeIterator(header->n, 0);
            range.size = 0;
            range.shape = XYZ(0, 0, 0);
            return range;
        }
        range.b = CubeIterator(header->n, filePointer + shapes[i].offset);
        range.e = CubeIterator(header->n, filePointer + shapes[i].offset + shapes[i].size);
        range.size = shapes[i].size / (header->n * sizeof(XYZ));
        range.shape = XYZ(shapes[i].dim0, shapes[i].dim1, shapes[i].dim2);
        return range;
    }
    auto size() { return header->numPolycubes; };
    auto numShapes() { return header->numShapes; };

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

#endif
