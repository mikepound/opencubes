#include <gtest/gtest.h>

#include "rotations.hpp"

TEST(RotationsTests, TestRotateDoesNotThrow) {
    XYZ shape = XYZ(1, 1, 1);
    Cube cube = Cube{{XYZ(0, 0, 0)}};
    for (int i = 0; i < 24; i++) {
        Cube dest(cube.size());
        EXPECT_NO_THROW(Rotations::rotate(i, shape, cube, dest));
    }
}

TEST(RotationsTests, TestRotationsMatchesExpectation) {
    XYZ shape = XYZ(2, 1, 1);
    Cube cube = {XYZ(0, 0, 0), XYZ(1, 0, 0)};
    XYZ expected_shapes[24] = {XYZ(2, 1, 1), XYZ(2, 1, 1), XYZ(2, 1, 1), XYZ(2, 1, 1), XYZ(2, 1, 1), XYZ(2, 1, 1), XYZ(2, 1, 1), XYZ(2, 1, 1),
                               XYZ(1, 2, 1), XYZ(1, 2, 1), XYZ(1, 2, 1), XYZ(1, 2, 1), XYZ(1, 1, 2), XYZ(1, 1, 2), XYZ(1, 1, 2), XYZ(1, 1, 2),
                               XYZ(1, 2, 1), XYZ(1, 2, 1), XYZ(1, 2, 1), XYZ(1, 2, 1), XYZ(1, 1, 2), XYZ(1, 1, 2), XYZ(1, 1, 2), XYZ(1, 1, 2)};
    Cube expected_cubes[24] = {
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {},
        {XYZ(1, 1, 0), XYZ(1, 1, 1)},
        {XYZ(1, 0, 2), XYZ(1, 0, 1)},
        {XYZ(0, 1, 2), XYZ(0, 1, 1)},
        {XYZ(0, 0, 0), XYZ(0, 0, 1)},
        {},
        {},
        {},
        {},
        {XYZ(1, 1, 2), XYZ(1, 1, 1)},
        {XYZ(1, 0, 0), XYZ(1, 0, 1)},
        {XYZ(0, 1, 0), XYZ(0, 1, 1)},
        {XYZ(0, 0, 2), XYZ(0, 0, 1)},
    };

    std::stringstream shapes;
    std::stringstream cubes;
    for (int i = 0; i < 24; i++) {
        Cube rotated(cube.size());
        auto [res, ok] = Rotations::rotate(i, shape, cube, rotated);
        EXPECT_EQ(res, expected_shapes[i]);
        if (ok) {
            EXPECT_EQ(rotated, expected_cubes[i]);
        }
    }
}
