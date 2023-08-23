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
    shape = tuple(el+2 for el in cube.shape)
    output_cube=np.zeros(shape,dtype=cube.dtype)
    output_cube[1:-1,1:-1,1:-1]=cube
    cube=output_cube.copy()

    xs, ys, zs = cube.nonzero()
    output_cube[xs+1, ys, zs] = 1
    output_cube[xs-1, ys, zs] = 1
    output_cube[xs, ys+1, zs] = 1
    output_cube[xs, ys-1, zs] = 1
    output_cube[xs, ys, zs+1] = 1
    output_cube[xs, ys, zs-1] = 1

    exp = (output_cube ^ cube).nonzero()

    for (x, y, z) in zip(*exp):
        new_cube = cube.copy()
        new_cube[x, y, z] = 1
        xl = 0 if x==0 else 1
        yl = 0 if y==0 else 1
        zl = 0 if z==0 else 1
        xr = cube.shape[0] - (not x==cube.shape[0]-1)
        yr = cube.shape[1] - (not y==cube.shape[1]-1)
        zr = cube.shape[2] - (not z==cube.shape[2]-1)
       
        yield new_cube[xl:xr,yl:yr,zl:zr]
