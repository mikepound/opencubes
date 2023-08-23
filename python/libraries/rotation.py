import numpy as np
from typing import Generator

def single_axis_rotation(polycube, axes):
    """Yield four rotations of the given 3d array in the plane spanned by the given axes.
    For example, a rotation in axes (0,1) is a rotation around axis 2"""
    for i in range(4):
        yield np.rot90(polycube, i, axes)

def all_rotations(polycube: np.ndarray) -> Generator[np.ndarray, None, None]:
    """
    Calculates all rotations of a polycube.

    Adapted from https://stackoverflow.com/questions/33190042/how-to-calculate-all-24-rotations-of-3d-array.
    This function computes all 24 rotations around each of the axis x,y,z. It uses numpy operations to do this, to avoid unecessary copies.
    The function returns a generator, to avoid computing all rotations if they are not needed.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    generator(np.array): Yields new rotations of this cube about all axes

    """

    # 4 rotations about axis 0
    yield from single_axis_rotation(polycube, (1, 2))

    # rotate 180 about axis 1, 4 rotations about axis 0
    yield from single_axis_rotation(np.rot90(polycube, 2, axes=(0, 2)), (1, 2))

    # rotate 90 or 270 about axis 1, 8 rotations about axis 2
    yield from single_axis_rotation(np.rot90(polycube, axes=(0, 2)), (0, 1))
    yield from single_axis_rotation(np.rot90(polycube, -1, axes=(0, 2)), (0, 1))

    # rotate about axis 2, 8 rotations about axis 1
    yield from single_axis_rotation(np.rot90(polycube, axes=(0, 1)), (0, 2))
    yield from single_axis_rotation(np.rot90(polycube, -1, axes=(0, 1)), (0, 2))

RotationIndexes={}

def get_canon_shape(polycube):
    return tuple(sorted(polycube.shape,reverse=True))


def all_rotations_fast(polycube: np.ndarray) -> Generator[np.ndarray, None, None]:
    orderedShape = get_canon_shape(polycube)
    if polycube.shape in RotationIndexes:
        ind = RotationIndexes[polycube.shape]
        return polycube.ravel()[ind].reshape((len(ind),)+orderedShape)
    else:
        n1,n2,n3 = polycube.shape
        vec = np.arange(n1*n2*n3).reshape(polycube.shape)
        uniqueRotations = set()
        rotations = list()

        def func(el):
            s = el.shape
            el = tuple(el.ravel().tolist())
            if not el in uniqueRotations and s == orderedShape:
                uniqueRotations.add(el)
                rotations.append(el)

        # 4 rotations about axis 0
        for el in single_axis_rotation(vec, (1, 2)):
            func(el)

        # rotate 180 about axis 1, 4 rotations about axis 0
        for el in single_axis_rotation(np.rot90(vec, 2, axes=(0, 2)), (1, 2)):
            func(el)

        # rotate 90 or 270 about axis 1, 8 rotations about axis 2
        for el in single_axis_rotation(np.rot90(vec, axes=(0, 2)), (0, 1)):
            func(el)
        for el in single_axis_rotation(np.rot90(vec, -1, axes=(0, 2)), (0, 1)):
            func(el)

        # rotate about axis 2, 8 rotations about axis 1
        for el in single_axis_rotation(np.rot90(vec, axes=(0, 1)), (0, 2)):
            func(el)
        for el in single_axis_rotation(np.rot90(vec, -1, axes=(0, 1)), (0, 2)):
            func(el)
            
        RotationIndexes[polycube.shape] = np.stack(rotations, axis=0)
        return polycube.ravel()[RotationIndexes[polycube.shape]].reshape((len(rotations),)+orderedShape)
