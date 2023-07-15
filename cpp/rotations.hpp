#pragma once
#ifndef OPENCUBES_ROTATIONS_HPP
#define OPENCUBES_ROTATIONS_HPP
#include <array>
#include <vector>

#include "structs.hpp"

struct Rotations {
    // ix,iy,iz,  sx,sy,sz. new component index and sign
    static constexpr std::array<int, 6> LUT[] = {
        {0, 1, 2, 1, 1, 1},  // identity
        {0, 1, 2, -1, -1, 1}, {0, 1, 2, -1, 1, -1},  {0, 1, 2, 1, -1, -1}, {0, 2, 1, -1, -1, -1}, {0, 2, 1, -1, 1, 1},  {0, 2, 1, 1, -1, 1},
        {0, 2, 1, 1, 1, -1},  {1, 0, 2, -1, -1, -1}, {1, 0, 2, -1, 1, 1},  {1, 0, 2, 1, -1, 1},   {1, 0, 2, 1, 1, -1},  {1, 2, 0, -1, -1, 1},
        {1, 2, 0, -1, 1, -1}, {1, 2, 0, 1, -1, -1},  {1, 2, 0, 1, 1, 1},   {2, 0, 1, -1, -1, 1},  {2, 0, 1, -1, 1, -1}, {2, 0, 1, 1, -1, -1},
        {2, 0, 1, 1, 1, 1},   {2, 1, 0, -1, -1, -1}, {2, 1, 0, -1, 1, 1},  {2, 1, 0, 1, -1, 1},   {2, 1, 0, 1, 1, -1},
    };
    static std::pair<XYZ, std::vector<XYZ>> rotate(int i, std::array<int, 3> shape, const Cube &orig) {
        const auto L = LUT[i];
        XYZ out_shape{shape[L[0]], shape[L[1]], shape[L[2]]};
        if (out_shape.x > out_shape.y || out_shape.y > out_shape.z) return {out_shape, {}};  // return here because violating shape
        std::vector<XYZ> res;
        res.reserve(orig.size());
        for (const auto &o : orig) {
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
            res.emplace_back(next);
        }
        return {out_shape, res};
    }
};
#endif
