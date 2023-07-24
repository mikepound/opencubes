from enum import Enum
from io import IOBase
from dataclasses import dataclass
from typing import Generator
import leb128
import math
from io import BytesIO
import gzip

magic_string = bytes.fromhex('CBECCBEC')

class Orientation(Enum):
    UNSORTED = 0
    BITWISE_HIGHEST_VALUE = 1

class Compression(Enum):
    NO_COMPRESSION = 0
    GZIP_COMPRESSION = 1

@dataclass
class Polycubes():
    orientation: Orientation
    polycubes: list[bytes]

vlq_num_mask = 0b01111111
vlq_continue_mask = 1<<7

def vlq_is_complete(byte) -> bool:
    return (byte | vlq_continue_mask)

def write(fp: IOBase, orientation: Orientation, polycubes: list[bytes], compression: Compression = Compression.NO_COMPRESSION) -> None:
    header = magic_string
    header += int(orientation.value).to_bytes(1, 'little')
    header += int(compression.value).to_bytes(1, 'little')
    header += leb128.u.encode(len(polycubes))
    fp.write(header)
    if(compression == Compression.GZIP_COMPRESSION):
        total_data = bytearray()
        for polycube in polycubes:
            total_data.extend(polycube)
        fp.write(gzip.compress(total_data, 5))
    else:
        for polycube in polycubes:
            fp.write(polycube)

def read_block(fp: IOBase) -> bytes:
    shape = fp.read(3)
    size = math.ceil((shape[0] * shape[1] * shape[2]) / 8)
    body = fp.read(size)
    return shape + body


def read(fp: IOBase, ignore_orientation:bool = False) -> Polycubes:
    magic = fp.read(4)
    if(magic != magic_string):
        raise ValueError("provided file does not have a valid pcube header")
    
    orientation = int.from_bytes(fp.read(1),'little')
    if(ignore_orientation):
        orientation = 0
    else:
        if(orientation not in [e.value for e in Orientation]):
            raise ValueError("provided file uses unsuported orientations")
    orientation = Orientation(orientation)

    compression = int.from_bytes(fp.read(1), 'little')
    if(compression not in [e.value for e in Compression]):
        raise ValueError("provided file uses unsuported compression")
    compression = Compression(compression)
    
    n_cubes, read = leb128.u.decode_reader(fp)

    use_fp = fp
    if (compression == Compression.GZIP_COMPRESSION):
        use_fp = BytesIO()
        use_fp.write(gzip.decompress(fp.read()))
        use_fp.seek(0)

    cubes = []

    if(n_cubes == 0):
        while(fp.readable()):
            cubes.append(read_block(use_fp))
    else:
        for n in range(0, n_cubes):
            cubes.append(read_block(use_fp))
    
    return Polycubes(orientation=orientation, polycubes=cubes)