#pragma once
#ifndef OPENCUBES_NEWCACHE_HPP
#define OPENCUBES_NEWCACHE_HPP
#include <cstring>
#include <string>

#include "cube.hpp"
#include "hashes.hpp"
#include "mapped_file.hpp"

class Workset;

class CubeIterator {
   public:
    using iterator_category = std::forward_iterator_tag;
    using difference_type = std::ptrdiff_t;
    using value_type = Cube;
    using pointer = Cube*;    // or also value_type*
    using reference = Cube&;  // or also value_type&

    // constructor
    CubeIterator(uint32_t _n, const XYZ* ptr) : n(_n), m_ptr(ptr) {}

    // invalid iterator (can't deference)
    explicit CubeIterator() : n(0), m_ptr(nullptr) {}

    // derefecence
    const value_type operator*() const { return Cube(m_ptr, n); }
    // pointer operator->() { return (pointer)m_ptr; }

    const XYZ* data() const { return m_ptr; }

    // Prefix increment
    CubeIterator& operator++() {
        m_ptr += n;
        return *this;
    }

    CubeIterator& operator+=(int incr) {
        m_ptr += n * incr;
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
    //friend class Workset;

   private:
    uint32_t n;
    const XYZ* m_ptr;
};

class ShapeRange {
   public:
    ShapeRange(const XYZ* start, const XYZ* stop, uint64_t _cubeLen, XYZ _shape)
        : b(_cubeLen, start), e(_cubeLen, stop), size_(std::distance(start, stop) / _cubeLen), shape_(_shape) {}

    CubeIterator begin() { return b; }
    CubeIterator end() { return e; }

    XYZ& shape() { return shape_; }
    auto size() const { return size_; }

   private:
    CubeIterator b, e;
    uint64_t size_;
    XYZ shape_;
};

class ICache {
   public:
    virtual ~ICache(){};
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
    void unload();

    size_t size() override { return header->numPolycubes; };
    uint32_t numShapes() override { return header->numShapes; };
    operator bool() { return fileLoaded_; }

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

    // Do begin() and end() make sense for CacheReader
    // If the cache file provides data for more than single shape?
    // The data might not even be mapped contiguously to save memory.
    /*CubeIterator begin() {
        const uint8_t* start = filePointer + shapes[0].offset;
        return CubeIterator(header->n, (const XYZ*)start);
    }

    CubeIterator end() {
        const uint8_t* stop = filePointer + shapes[0].offset + header->numPolycubes * header->n * XYZ_SIZE;
        return CubeIterator(header->n, (const XYZ*)stop);
    }*/

    // get shapes at index [0, numShapes()[
    ShapeRange getCubesByShape(uint32_t i) override;

   private:
    std::shared_ptr<mapped::file> file_;
    std::unique_ptr<const mapped::struct_region<Header>> header_;
    std::unique_ptr<const mapped::array_region<ShapeEntry>> shapes_;
    std::unique_ptr<const mapped::array_region<XYZ>> xyz_;

    std::string path_;
    bool fileLoaded_;
    const Header dummyHeader;
    const Header* header;
    const ShapeEntry* shapes;
};

class FlatCache : public ICache {
    std::vector<XYZ> allXYZs;
    std::vector<ShapeRange> shapes;
    uint8_t n = 0;

   public:
    FlatCache() {}
    FlatCache(Hashy& hashes, uint8_t n) : n(n) {
        allXYZs.reserve(hashes.size() * n);
        shapes.reserve(hashes.byshape.size());
        // std::printf("Flatcache %d %p %p\n", n, (void*)allXYZs.data(), (void*)shapes.data());
        for (auto& [shape, set] : hashes.byshape) {
            auto begin = allXYZs.data() + allXYZs.size();
            for (auto& subset : set.byhash) {
                for (auto& cube : subset.set)
                    // allXYZs.emplace_back(allXYZs.end(), subset.set.begin(), subset.set.end());
                    std::copy(cube.begin(), cube.end(), std::back_inserter(allXYZs));
            }
            auto end = allXYZs.data() + allXYZs.size();
            // std::printf("  SR %p %p\n", (void*)begin, (void*)end);
            shapes.emplace_back(begin, end, n, shape);
        }
    }
    ShapeRange getCubesByShape(uint32_t i) override {
        if (i >= shapes.size()) return ShapeRange{nullptr, nullptr, 0, XYZ(0, 0, 0)};
        return shapes[i];
    };
    uint32_t numShapes() override { return shapes.size(); };
    size_t size() override { return allXYZs.size() / n / sizeof(XYZ); }
};

#endif
