#pragma once

#include <vector>
#include <array>

#include "structs.hpp"

struct Rotations
{
    static constexpr array<int, 6> LUT[] = {
        {0, 1, 2, -1, -1, 1},
        {0, 1, 2, -1, 1, -1},
        {0, 1, 2, 1, -1, -1},
        {0, 1, 2, 1, 1, 1},
        {0, 2, 1, -1, -1, -1},
        {0, 2, 1, -1, 1, 1},
        {0, 2, 1, 1, -1, 1},
        {0, 2, 1, 1, 1, -1},
        {1, 0, 2, -1, -1, -1},
        {1, 0, 2, -1, 1, 1},
        {1, 0, 2, 1, -1, 1},
        {1, 0, 2, 1, 1, -1},
        {1, 2, 0, -1, -1, 1},
        {1, 2, 0, -1, 1, -1},
        {1, 2, 0, 1, -1, -1},
        {1, 2, 0, 1, 1, 1},
        {2, 0, 1, -1, -1, 1},
        {2, 0, 1, -1, 1, -1},
        {2, 0, 1, 1, -1, -1},
        {2, 0, 1, 1, 1, 1},
        {2, 1, 0, -1, -1, -1},
        {2, 1, 0, -1, 1, 1},
        {2, 1, 0, 1, -1, 1},
        {2, 1, 0, 1, 1, -1},
    };
    static std::vector<XYZ> rotate(int i, std::array<int, 3> shape, const std::vector<XYZ> &orig)
    {
        std::vector<XYZ> res;
        res.reserve(orig.size());
        const auto L = LUT[i];
        for (const auto &o : orig)
        {
            XYZ next;
            if (L[3] < 0)
                next.x = shape[L[0]] - o.data[L[0]];
            else
                next.x = o.data[L[0]];

            if (L[4] < 0)
                next.y = shape[L[1]] - o.data[L[1]];
            else
                next.y = o.data[L[1]];

            if (L[5] < 0)
                next.z = shape[L[2]] - o.data[L[2]];
            else
                next.z = o.data[L[2]];
            res.push_back(next);
        }
        return res;
    }
};