import numpy as np


def pack(polycube: np.ndarray) -> int:
    """
    Converts a 3D ndarray into a single unsigned integer for quick hashing and efficient storage

    Converts a {0,1} nd array into a single unique large integer

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    int: a unique integer hash

    """

    pack_cube = np.packbits(polycube.flatten(), bitorder='big')
    cube_hash = 0
    for index in polycube.shape:
        cube_hash = (cube_hash << 8) + int(index)
    for part in pack_cube:
        cube_hash = (cube_hash << 8) + int(part)
    return cube_hash


def unpack(cube_hash: int) -> np.ndarray:
    """
    Converts a single integer back into a 3D ndarray


    Parameters:
    cube_hash (int): a unique integer hash

    Returns:
    np.array: 3D Numpy byte array where 1 values indicate polycube positions

    """
    parts = []
    while (cube_hash):
        parts.append(cube_hash % 256)
        cube_hash >>= 8
    parts = parts[::-1]
    shape = (parts[0], parts[1], parts[2])
    data = parts[3:]
    size = shape[0] * shape[1] * shape[2]
    raw = np.unpackbits(np.array(data, dtype=np.uint8), bitorder='big')
    final = raw[0:size].reshape(shape)
    return final
