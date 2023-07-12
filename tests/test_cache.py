import unittest
import os
from libraries.cache import get_cache, save_cache
from numpy.testing import assert_array_equal
from utils import get_test_data

class CachingTests(unittest.TestCase):

    def test_cache_consistency(self):
        test_data = get_test_data()
        
        save_cache("test_temp", test_data)
        reloaded_data = get_cache("test_temp")
        
        for test, reloaded in zip(test_data, reloaded_data):
            assert_array_equal(test, reloaded)

    @classmethod
    def tearDownClass(cls):
        expected_test_file_name = "cubes_test_temp.npy"
        if os.path.exists(expected_test_file_name):
            os.remove(expected_test_file_name)