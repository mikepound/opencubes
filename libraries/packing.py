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

    # # Previous implementation:
    # pack_cube = np.packbits(polycube.flatten(), bitorder='big')
    # cube_hash = 0
    # for index in polycube.shape:
    #     cube_hash = (cube_hash << 8) + int(index)
    # for part in pack_cube:
    #     cube_hash = (cube_hash << 8) + int(part)
    # return cube_hash

    # # dtype should be np.int8: (commented out for efficiency)
    # if polycube.dtype != np.int8:
    #     raise TypeError("Polycube must be of type np.int8")

    # pack cube
    data = polycube.tobytes() + polycube.shape[0].to_bytes(1, 'big') \
                              + polycube.shape[1].to_bytes(1, 'big') \
                              + polycube.shape[2].to_bytes(1, 'big')
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
    shape = (cube_id[-3], cube_id[-2], cube_id[-1])

    # Create ndarray from byte data
    polycube = np.frombuffer(cube_id[:-3], dtype=np.int8)
    polycube = polycube.reshape(shape)
    return polycube

