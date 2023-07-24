import unittest
from libraries.packing import pack
from libraries.pcube import read, write, Orientation, Compression
from io import BytesIO
from .utils import get_test_data

class PcubeTests(unittest.TestCase):
    def test_pcube_integrity(self):
        test_data = get_test_data()
        packed = [pack(cube) for cube in test_data]
        
        with BytesIO() as pcube_stream:
            write(pcube_stream, polycubes=packed, orientation=Orientation.UNSORTED, compression=Compression.NO_COMPRESSION)
            pcube_stream.seek(0)
            result = read(pcube_stream)
        
        self.assertEqual(packed, result.polycubes)

    def test_pcube_compressed_integrity(self):
        test_data = get_test_data()
        packed = [pack(cube) for cube in test_data]
        
        with BytesIO() as pcube_stream:
            write(pcube_stream, polycubes=packed, orientation=Orientation.UNSORTED, compression=Compression.GZIP_COMPRESSION)
            pcube_stream.seek(0)
            result = read(pcube_stream)
        
        self.assertEqual(packed, result.polycubes)