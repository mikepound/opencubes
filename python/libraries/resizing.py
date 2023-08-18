import numpy as np
from typing import Generator


def crop_cube(cube: np.ndarray) -> np.ndarray:
    """
    Crops an np.array to have no all-zero padding around the edge.

    Given in https://stackoverflow.com/questions/39465812/how-to-crop-zero-edges-of-a-numpy-array

    Parameters:
    cube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    np.array: Cropped 3D Numpy byte array equivalent to cube, but with no zero padding

    """
    for i in range(cube.ndim):
        cube = np.swapaxes(cube, 0, i)
        nonzero_indices = np.any(cube != 0, axis=tuple(range(1, cube.ndim)))
        cube = cube[nonzero_indices]
        cube = np.swapaxes(cube, 0, i)
    return cube


def fix_axis(cube):
    """
    Rotate cube to make boundary sizes in sorted order.

    Ex : if input cube.shape = (4, 1, 2) this method rotate it to be (1, 2, 4)

    Parameters:
    cube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    np.array: Cropped 3D Numpy byte array equivalent to cube, but with no zero padding

    """
    if cube.shape == tuple(sorted(cube.shape)):
        return cube

    if cube.shape == tuple(sorted(cube.shape, reverse=True)):
        return np.rot90(cube, 1, (0, 2))

    if cube.shape[0] == cube.shape[1] or cube.shape[1] == cube.shape[2]:
        if cube.shape[0] < cube.shape[2]:
            return cube
        else:
            return np.rot90(cube, 1, (0, 2))

    if cube.shape[0] == cube.shape[2]:
        if cube.shape[0] < cube.shape[1]:
            return np.rot90(cube, 1, (1, 2))
        else:
            return np.rot90(cube, 1, (0, 1))

    if cube.shape[0] < cube.shape[2] < cube.shape[1]:
        return np.rot90(cube, 1, (1, 2))

    if cube.shape[1] < cube.shape[2] < cube.shape[0]:
        return np.rot90(np.rot90(cube, 1, (0, 1)), 1, (1, 2))

    if cube.shape[1] < cube.shape[0] < cube.shape[2]:
        return np.rot90(cube, 1, (0, 1))

    if cube.shape[2] < cube.shape[0] < cube.shape[1]:
        return np.rot90(np.rot90(cube, 1, (1, 2)), 1, (0, 1))

    print("error", cube.shape)
    exit(2)

def expand_cube(cube: np.ndarray) -> Generator[np.ndarray, None, None]:
    """
    Expands a polycube by adding single blocks at all valid locations.

    Calculates all valid new positions of a polycube by shifting the existing cube +1 and -1 in each dimension.
    New valid cubes are returned via a generator function, in case they are not all needed.

    Parameters:
    cube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    generator(np.array): Yields new polycubes that are extensions of cube

    """
    cube = np.pad(cube, 1, 'constant', constant_values=0)
    output_cube = np.array(cube)

    xs, ys, zs = cube.nonzero()
    output_cube[xs+1, ys, zs] = 1
    output_cube[xs-1, ys, zs] = 1
    output_cube[xs, ys+1, zs] = 1
    output_cube[xs, ys-1, zs] = 1
    output_cube[xs, ys, zs+1] = 1
    output_cube[xs, ys, zs-1] = 1

    exp = (output_cube ^ cube).nonzero()

    for (x, y, z) in zip(exp[0], exp[1], exp[2]):
        new_cube = np.array(cube)
        new_cube[x, y, z] = 1
        yield fix_axis(crop_cube(new_cube))
