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
    output_cube[xs, ys, zs] = 0

    exp = output_cube.nonzero()
    bounds=list()
    bound=np.empty_like(exp[0])
    for i in range(3):
        ind = exp[i]==0
        bound[ind]=0
        bound[~ind]=1
        bounds.append(bound.copy())
        ind=exp[i]==cube.shape[i]-1
        bound[ind]=cube.shape[i]
        bound[~ind]=cube.shape[i]-1
        bounds.append(bound.copy())
    
    n=len(exp[0])
    for i in range(n):
        new_cube = cube.copy()
        new_cube[exp[0][i], exp[1][i], exp[2][i]] = 1
        yield new_cube[bounds[0][i]:bounds[1][i],bounds[2][i]:bounds[3][i],bounds[4][i]:bounds[5][i]]
    
        
def test_expand():
    """
    Function to test the performance of the expand_cube() function
    """
    from time import perf_counter
    
    n=1000
    shape=(4,3,2)
    polycubes = (np.random.random((n,)+shape)>0.5).astype(np.byte)
    now=perf_counter()
    for cube in polycubes:
        res=list(expand_cube(cube))
        # res=list(expand_cube_fast(cube))
    print(perf_counter()-now)
    
if __name__ == "__main__":
    test_expand()