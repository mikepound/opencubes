import math
import numpy as np
import matplotlib.pyplot as plt

# # Code for if you want to generate pictures of the sets of cubes. Will work up to about n=8, before there are simply too many!
# # Could be adapted for larger cube sizes by splitting the dataset up into separate images.


def render_shapes(shapes: list[np.ndarray], path: str):
    n = len(shapes)
    dim = max(max(a.shape) for a in shapes)
    i = math.isqrt(n) + 1
    voxel_dim = dim * i
    voxel_array = np.zeros((voxel_dim + i, voxel_dim + i, dim), dtype=np.byte)
    pad = 1
    for idx, shape in enumerate(shapes):
        x = (idx % i) * dim + (idx % i)
        y = (idx // i) * dim + (idx // i)
        xpad = x * pad
        ypad = y * pad
        s = shape.shape
        voxel_array[x:x + s[0], y:y + s[1], 0: s[2]] = shape

    # voxel_array = crop_cube(voxel_array)
    colors = np.empty(voxel_array.shape, dtype=object)
    colors[:] = '#FFD65DC0'

    ax = plt.figure(figsize=(20, 16), dpi=600).add_subplot(projection='3d')
    ax.voxels(voxel_array, facecolors=colors, edgecolor='k', linewidth=0.1)

    ax.set_xlim([0, voxel_array.shape[0]])
    ax.set_ylim([0, voxel_array.shape[1]])
    ax.set_zlim([0, voxel_array.shape[2]])
    plt.axis("off")
    ax.set_box_aspect((1, 1, voxel_array.shape[2] / voxel_array.shape[0]))
    plt.savefig(path + ".png", bbox_inches='tight', pad_inches=0)
