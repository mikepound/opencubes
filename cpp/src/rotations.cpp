#include "rotations.hpp"

#include <array>
#include <vector>

#include "cube.hpp"

std::pair<XYZ, bool> Rotations::rotate(int i, XYZ shape, const Cube &orig, Cube &dest) {
    const auto L = LUT[i];
    XYZ out_shape{shape[L[0]], shape[L[1]], shape[L[2]]};
    if (out_shape.x() > out_shape.y() || out_shape.y() > out_shape.z()) {
        return {out_shape, false};  // return here because violating shape
    }
    auto put = dest.begin();
    for (const auto &o : orig) {
        XYZ next;
        if (L[3] < 0)
            next.x() = shape[L[0]] - o.data[L[0]];
        else
            next.x() = o.data[L[0]];

        if (L[4] < 0)
            next.y() = shape[L[1]] - o.data[L[1]];
        else
            next.y() = o.data[L[1]];

        if (L[5] < 0)
            next.z() = shape[L[2]] - o.data[L[2]];
        else
            next.z() = o.data[L[2]];
        *put++ = next;
    }
    return {out_shape, true};
}
