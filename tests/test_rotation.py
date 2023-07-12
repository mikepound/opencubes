import unittest
import numpy as np
from libraries.rotation import all_rotations
from utils import get_test_data

class RotatingTests(unittest.TestCase):
    def test_rotate_quantity(self):
        test_data = get_test_data()
        for polycube in test_data:
            rots = all_rotations(polycube)
            self.assertEqual(len(list(rots)), 24, "all_rotations failed to produce 24 rotations")

    def test_rotate_symetric(self):
        # tests that all rotations of any given rotation from an all rotations set, itself is in the all rotations set.
        # e.g. repeatedly rotating doesnt change the fundemental 24 rotations of a given polycube
        test_data = get_test_data()
        for polycube in test_data:
            base_polycube_rotations = list(all_rotations(polycube))
            for base_cube_rotation in base_polycube_rotations:
                for rotated_cube_rotation in all_rotations(base_cube_rotation):
                    found_match = False
                    for inner_base_polycube_rotation in base_polycube_rotations:
                        if np.array_equal(rotated_cube_rotation, inner_base_polycube_rotation):
                            found_match = True
                            break
                    self.assertTrue(found_match, "rotation of a rotated polycube wasnt in the set of all rotations for the initial pollycube, rotating it twice has changed its shape")