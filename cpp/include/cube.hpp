#pragma once
#ifndef OPENCUBES_CUBE_HPP
#define OPENCUBES_CUBE_HPP

#include <algorithm>
#include <cstdint>
#include <memory>
#include <unordered_set>
#include <vector>
#include <atomic>

#include "utils.hpp"

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
    friend XYZ operator+(const XYZ &a, const XYZ &b) {
        XYZ ret = a;
        ret += b;
        return ret;
    }
    void operator+=(const XYZ &b) {
        data[0] += b.data[0];
        data[1] += b.data[1];
        data[2] += b.data[2];
    }
};

struct HashXYZ {
    size_t operator()(const XYZ &p) const { return (uint32_t)p; }
};

using XYZSet = std::unordered_set<XYZ, HashXYZ, std::equal_to<XYZ>>;

struct Cube {
   private:
    // cube memory is stored two ways:
    // normal, new'd buffer: is_shared == false
    // shared, external memory: is_shared == true

    struct bits_t {
        uint64_t is_shared : 1;
        uint64_t size : 7;   // MAX 127
        uint64_t addr : 56;  // low 56-bits of memory address.
    };
    // fields
    bits_t fields;

    static_assert(sizeof(bits_t) == sizeof(void*));
    // extract the pointer from bits_t
    static XYZ *get(bits_t key) {
        // pointer bit-hacking:
        uint64_t addr = key.addr;
        return reinterpret_cast<XYZ *>(addr);
    }

    static bits_t put(bool is_shared, int size, XYZ *addr) {
        // mask off top byte from the memory address to fit it into bits_t::addr
        // on x86-64 it is not used by the hardware (yet).
        // This hack actually saves 8 bytes because previously
        // the uint8_t caused padding to 16 bytes.
        // @note if we get segfaults dereferencing get(fields)
        // then this is the problem and this hack must be undone.
        uint64_t tmp = reinterpret_cast<uint64_t>((void *)addr);
        tmp &= 0xffffffffffffff;
        bits_t bits;
        bits.addr = tmp;
        bits.is_shared = is_shared;
        bits.size = size;
        return bits;
    }
   public:
    // Empty cube
    Cube() : fields{put(0, 0, nullptr)} {}

    // Cube with N capacity
    explicit Cube(uint8_t N) : fields{put(0,N, new XYZ[N])} {}

    // Construct from pieces
    Cube(std::initializer_list<XYZ> il) : Cube(il.size()) { std::copy(il.begin(), il.end(), begin()); }

    // Construct from range.
    Cube(const XYZ *start, const XYZ *end) : Cube(std::distance(start, end)) { std::copy(start, end, begin()); }

    // Construct from external source.
    // Cube shares this the memory until modified.
    // Caller guarantees the memory given will live longer than *this
    Cube(const XYZ *start, uint8_t n) : fields{put(1,n,const_cast<XYZ*>(start))} {}

    // Copy ctor.
    Cube(const Cube &copy) : Cube(copy.size()) { std::copy(copy.begin(), copy.end(), begin()); }

    ~Cube() {
        bits_t bits = fields;
        if (!bits.is_shared) {
            delete[] get(bits);
        }
    }
    friend void swap(Cube &a, Cube &b) {
        using std::swap;
        bits_t abits = a.fields;
        bits_t bbits = b.fields;
        a.fields = bbits;
        b.fields = abits;
    }

    Cube(Cube &&mv) : Cube() { swap(*this, mv); }

    Cube &operator=(const Cube &copy) {
        Cube tmp(copy);
        swap(*this, tmp);
        return *this;
    }

    Cube &operator=(Cube &&mv) {
        swap(*this, mv);
        return *this;
    }

    size_t size() const { return fields.size; }

    XYZ *data() {
		return get(fields);
	}

	const XYZ *data() const {
		return get(fields);
	}

    XYZ *begin() { return data(); }

    XYZ *end() { return data() + size(); }

    const XYZ *begin() const { return data(); }

    const XYZ *end() const { return data() + size(); }

    bool operator==(const Cube &rhs) const {
        if (size() != rhs.size()) return false;
        return std::mismatch(begin(), end(), rhs.begin()).first == end();
    }

    bool operator<(const Cube &b) const {
        if (size() != b.size()) return size() < b.size();
        auto [aa, bb] = std::mismatch(begin(), end(), b.begin());
        if (aa == end()) {
            return false;
        } else {
            return *aa < *bb;
        }
    }

    void print() const {
        for (auto &p : *this) std::printf("  (%2d %2d %2d)\n\r", p.x(), p.y(), p.z());
    }
};

static_assert(sizeof(Cube) == 8, "Unexpected sizeof(Cube) for Cube");
static_assert(std::is_move_assignable_v<Cube>, "Cube must be moveable");
static_assert(std::is_swappable_v<Cube>, "Cube must swappable");

#endif
