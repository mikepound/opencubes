#pragma once
#ifndef OPENCUBES_ROTATIONS_HPP
#define OPENCUBES_ROTATIONS_HPP
#include <array>

#include "cube.hpp"

struct Rotations {
    // ix,iy,iz,  sx,sy,sz. new component index and sign
    static constexpr std::array<int, 6> LUT[] = {
        {0, 1, 2, 1, 1, 1},  // identity
        {0, 1, 2, -1, -1, 1}, {0, 1, 2, -1, 1, -1},  {0, 1, 2, 1, -1, -1}, {0, 2, 1, -1, -1, -1}, {0, 2, 1, -1, 1, 1},  {0, 2, 1, 1, -1, 1},
        {0, 2, 1, 1, 1, -1},  {1, 0, 2, -1, -1, -1}, {1, 0, 2, -1, 1, 1},  {1, 0, 2, 1, -1, 1},   {1, 0, 2, 1, 1, -1},  {1, 2, 0, -1, -1, 1},
        {1, 2, 0, -1, 1, -1}, {1, 2, 0, 1, -1, -1},  {1, 2, 0, 1, 1, 1},   {2, 0, 1, -1, -1, 1},  {2, 0, 1, -1, 1, -1}, {2, 0, 1, 1, -1, -1},
        {2, 0, 1, 1, 1, 1},   {2, 1, 0, -1, -1, -1}, {2, 1, 0, -1, 1, 1},  {2, 1, 0, 1, -1, 1},   {2, 1, 0, 1, 1, -1},
    };
    static std::pair<XYZ, bool> rotate(int i, XYZ shape, const Cube &orig, Cube &dest);
};
#endif
