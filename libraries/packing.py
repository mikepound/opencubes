import numpy as np
import math


def pack(polycube: np.ndarray) -> int:
    """
    Converts a 3D ndarray into a single unsigned integer for quick hashing and efficient storage

    Converts a {0,1} nd array into a single unique large integer

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    int: a unique integer hash

    """

    # pack_cube = np.packbits(polycube.flatten(), bitorder='big')
    # cube_hash = 0
    # for index in polycube.shape:
    #     cube_hash = (cube_hash << 8) + int(index)
    # for part in pack_cube:
    #     cube_hash = (cube_hash << 8) + int(part)
    # return cube_hash

    data = polycube.tobytes() + polycube.shape[0].to_bytes(1, 'big') + polycube.shape[1].to_bytes(1, 'big') + polycube.shape[2].to_bytes(1, 'big')
    return int.from_bytes(data, 'big')


def unpack(cube_hash: int) -> np.ndarray:
    """
    Converts a single integer back into a 3D ndarray


    Parameters:
    cube_hash (int): a unique integer hash

    Returns:
    np.array: 3D Numpy byte array where 1 values indicate polycube positions

    """

    length = math.ceil(math.log2(cube_hash))
    parts = cube_hash.to_bytes(length, byteorder='big')
    shape = (
        parts[-3],
        parts[-2],
        parts[-1],
    )
    size = shape[0] * shape[1] * shape[2]
    raw = np.frombuffer(parts[:-3], dtype=np.uint8)
    final = raw[(len(raw) - size):len(raw)].reshape(shape)
    return final
