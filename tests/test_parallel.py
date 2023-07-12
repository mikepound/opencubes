import unittest
import random
from libraries.parallel import init_parrelism, dispatch_tasks

def reverse_ints_task(args):
    (data, logger) = args
    return data[::-1]

class ParallelTests(unittest.TestCase):
    def _underlying_test(self, run_in_paralel):
        init_parrelism()
        random.seed(0) # determenistic random data generation
        test_data = [random.randint(0, 100) for x in range(0,763554)]

        results = dispatch_tasks(reverse_ints_task, test_data, run_in_paralel)
        self.assertEqual(test_data, results[::-1])

    def test_simple_serial_task(self):
        self._underlying_test(False)

    def test_simple_paralel_task(self):
        self._underlying_test(True)