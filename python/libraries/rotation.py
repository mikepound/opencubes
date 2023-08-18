import numpy as np
from typing import Generator


def all_rotations(polycube: np.ndarray) -> Generator[np.ndarray, None, None]:
    """
    Calculates rotations of a polycube when bounding box size in all axes are equal(Ex (3, 3, 3)).

    Adapted from https://stackoverflow.com/questions/33190042/how-to-calculate-all-24-rotations-of-3d-array.
    This function computes all 24 rotations around each of the axis x,y,z. It uses numpy operations to do this, to avoid unecessary copies.
    The function returns a generator, to avoid computing all rotations if they are not needed.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    generator(np.array): Yields new rotations of this cube about all axes

    """
    def single_axis_rotation(polycube, axes):
        """Yield four rotations of the given 3d array in the plane spanned by the given axes.
        For example, a rotation in axes (0,1) is a rotation around axis 2"""
        for i in range(4):
            yield np.rot90(polycube, i, axes)

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


def one_diff_rot(cube, diff_axis, equal_axes):
    """
    Calculates rotations of a polycube when bounding box size in two axes are equal, but one is different. (Ex (2, 2, 3)).
    Only 8 rotations can be done without breaking the bounding box.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions
    diff_axis (integer): Axis which has different size
    equal_axes (integer array): Axes which have same size

    Returns:
    generator(np.array): Yields new rotations of this cube about all axes

    """
    for _ in range(0, 4):
        yield cube
        cube = np.rot90(cube, 1, equal_axes)

    cube = np.rot90(cube, 2, (diff_axis, equal_axes[0]))

    for _ in range(0, 4):
        yield cube
        cube = np.rot90(cube, 1, equal_axes)


def all_diff_rot(cube):
    """
    Calculates rotations of a polycube when bounding box size in all axes are different. (Ex (1, 4, 6)).
    Only 4 rotations can be done without breaking the bounding box.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    generator(np.array): Yields new rotations of this cube about all axes

    """
    yield cube
    yield np.rot90(cube, 2, (0, 1))
    yield np.rot90(cube, 2, (0, 2))
    yield np.rot90(cube, 2, (1, 2))

