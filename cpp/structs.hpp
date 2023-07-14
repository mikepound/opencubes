#pragma once
#include <vector>

using namespace std;
// #define DBG 1
struct XYZ
{
    union
    {
        struct
        {
            int8_t x, y, z, res;
        };
        int8_t data[4];
        int32_t joined;
    };
    explicit XYZ(int a = 0, int b = 0, int c = 0) : x(a), y(b), z(c), res(0) {}
    bool operator==(const XYZ &rhs) const { return joined == rhs.joined; }
    bool operator<(const XYZ &b) const { return joined < b.joined; }
};

struct Cube
{
    vector<XYZ> sparse;
    bool operator==(const Cube &rhs) const { return this->sparse == rhs.sparse; }
    bool operator<(const Cube &b) const
    {
        if (sparse.size() != b.sparse.size())
            return sparse.size() < b.sparse.size();
        for (int i = 0; i < sparse.size(); ++i)
        {
            if (sparse[i].joined < b.sparse[i].joined)
                return true;
            else if (sparse[i].joined > b.sparse[i].joined)
                return false;
        }
        return false;
    }

    void print()
    {
        for (auto &p : sparse)
            printf("  (%2d %2d %2d)\n\r", p.x, p.y, p.z);
    }
};

namespace std
{
    template <>
    struct hash<XYZ>
    {
        size_t operator()(const XYZ &x) const { return x.joined; }
    };

    template <>
    struct hash<Cube>
    {
        size_t operator()(const Cube &cube) const
        {
            // https://stackoverflow.com/questions/20511347/a-good-hash-function-for-a-vector/72073933#72073933
            std::size_t seed = cube.sparse.size();
            for (auto &p : cube.sparse)
            {
                auto x = std::hash<XYZ>()(p);
                // x = ((x >> 16) ^ x) * 0x45d9f3b;
                // x = ((x >> 16) ^ x) * 0x45d9f3b;
                // x = (x >> 16) ^ x;
                seed ^= x + 0x9e3779b9 + (seed << 6) + (seed >> 2);
            }
            return seed;
        }
    };
} // namespace std