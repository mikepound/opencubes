import unittest
from numpy.testing import assert_array_equal
from libraries.packing import pack, unpack
from utils import get_test_data

class PackingTests(unittest.TestCase):
    def test_pack_does_not_throw(self):
        test_data = get_test_data()
        for polycube in test_data:
            try:
                packed = pack(polycube)
            except:
                self.fail(f"pack threw on pollycube {polycube}")

    def test_unpack_does_not_throw(self):
        test_data = get_test_data()
        for polycube in test_data:
            packed = pack(polycube)
            try:
                unpacked = unpack(packed)
            except:
                self.fail(f"unpack threw on packing {packed} for pollycube {polycube}")

    def test_pack_hashable(self):
        test_data = get_test_data()
        for polycube in test_data:
            packed = pack(polycube)
            try:
                hash(packed)
            except:
                self.fail(f"packing of pollycube {polycube} isnt hashable")

    def test_pack_equitable(self):
        test_data = get_test_data()
        for polycube in test_data:
            packed = pack(polycube)
            self.assertEqual(packed, packed, "hash does not equal itself")

    def test_pack_unique(self):
        test_data = get_test_data()
        seen = set()
        for polycube in test_data:
            packed = pack(polycube)
            self.assertNotIn(packed, seen)
            seen.add(packed)

    def test_pack_symetric(self):
        test_data = get_test_data()
        for polycube in test_data:
            packed = pack(polycube)
            unpacked = unpack(packed)
            assert_array_equal(polycube, unpacked, f"packing of polycube isnt symetric, unpacked polycube {polycube} packed to {packed} which unpacked to {unpacked}")