import numpy as np
import math


def pack(polycube: np.ndarray) -> bytes:
    """
    Converts a 3D ndarray into a single bytes object that unique identifies 
    the polycube, is hashable, comparable, and allows to reconstruct the 
    original polycube ndarray.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions,
        and 0 values indicate empty space. Must be of type np.int8.

    Returns:
    cube_id (bytes): a bytes representation of the polycube

    """
    data =  polycube.shape[0].to_bytes(1, 'little') \
            + polycube.shape[1].to_bytes(1, 'little') \
            + polycube.shape[2].to_bytes(1, 'little') \
            + np.packbits(polycube.flatten(), bitorder='little').tobytes()
    return data


def unpack(cube_id: bytes) -> np.ndarray:
    """
    Converts a bytes object back into a 3D ndarray

    Parameters:
    cube_id (bytes): a unique bytes object

    Returns:
    polycube (np.array): 3D Numpy byte array where 1 values indicate 
        cube positions
        
    """
    # Extract shape information
    shape = (cube_id[0], cube_id[1], cube_id[2])
    size = shape[0] * shape[1] * shape[2]
    polycube = np.unpackbits(np.frombuffer(cube_id[3:], dtype=np.uint8), count=size, bitorder='little').reshape(shape)
    return polycube

