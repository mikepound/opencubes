import os
import numpy as np
import math
import argparse
from time import perf_counter
from libraries.cache import get_cache, save_cache, cache_exists
from libraries.cropping import crop_cube, expand_cube
from libraries.packing import pack, unpack
from libraries.parallel import dispatch_tasks, init_parrelism
from libraries.renderer import render_shapes
from libraries.rotation import all_rotations
from multiprocessing import Queue


def unpack_hashes_task(args: tuple[list[int], Queue]) -> np.ndarray:
    cube_hashes, logging_queue = args
    polycubes = []
    uid = os.getpid()

    n = 0
    log_base = max(math.ceil((len(cube_hashes)/ 1000)), 100) # send log updates every 0.1% or every 100, whichever is bigger
    for cube_hash in cube_hashes:
        polycubes.append(unpack(cube_hash))

        if (n % log_base == 0):
            if (logging_queue):
                logging_queue.put((uid, n, len(cube_hashes)))
            else:
                print(f'done {((n/len(cube_hashes)) * 100):.2f} %')
        n += 1

    if (logging_queue):
        logging_queue.put((uid, n, len(cube_hashes)))
    else:
        print(f'done {((n/len(cube_hashes)) * 100):.2f} %')
    return polycubes


def hash_cubes_task(args: tuple[list[np.ndarray], Queue]) -> list[int]:
    base_cubes, logging_queue = args
    # Empty list of new n-polycubes
    hashes = set()
    uid = os.getpid()

    n = 0
    log_base = max(math.ceil((len(base_cubes)/ 1000)), 100) # send log updates every 0.1% or every 100, whichever is bigger
    for base_cube in base_cubes:
        for new_cube in expand_cube(base_cube):
            cube_hash = get_canoincal_packing(new_cube)
            hashes.add(cube_hash)

        if (n % log_base == 0):
            if (logging_queue):
                logging_queue.put((uid, n, len(base_cubes)))
            else:
                print(f'done {((n/len(base_cubes)) * 100):.2f} %')
        n += 1

    if (logging_queue):
        logging_queue.put((uid, n, len(base_cubes)))
    else:
        print(f'done {((n/len(base_cubes)) * 100):.2f} %')

    return hashes


def generate_polycubes(n: int, use_cache: bool = False, enable_multicore: bool = False) -> list[np.ndarray]:
    """
    Generates all polycubes of size n

    Generates a list of all possible configurations of n cubes, where all cubes are connected via at least one face.
    Builds each new polycube from the previous set of polycubes n-1.
    Uses an optional cache to save and load polycubes of size n-1 for efficiency.

    Parameters:
    n (int): The size of the polycubes to generate, e.g. all combinations of n=4 cubes.

    Returns:
    list(np.array): Returns a list of all polycubes of size n as numpy byte arrays

    """
    if n < 1:
        return []
    elif n == 1:
        return [np.ones((1, 1, 1), dtype=np.byte)]
    elif n == 2:
        return [np.ones((2, 1, 1), dtype=np.byte)]

    if (use_cache and cache_exists(n)):
        results = get_cache(n)
        print(f"Got polycubes from cache n={n}\n")
    else:
        pollycubes = generate_polycubes(n-1, use_cache, enable_multicore)
        results = dispatch_tasks(hash_cubes_task, pollycubes, enable_multicore)
        print(f"Hashed polycubes n={n}\n")
        results = dispatch_tasks(unpack_hashes_task, results, enable_multicore)
        print(f"Generated polycubes from hash n={n}\n")

    if (use_cache and not cache_exists(n)):
        save_cache(n, results)

    return results


def get_canoincal_packing(polycube: np.ndarray) -> int:
    """
    Determines if a polycube has already been seen.

    Considers all possible rotations of a cube against the existing cubes stored in memory.
    Returns True if the cube exists, or False if it is new.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    boolean: True if polycube is already present in the set of all cubes so far.
    hash: the hash for this cube

    """
    max_hash = 0
    for cube_rotation in all_rotations(polycube):
        this_hash = pack(cube_rotation)
        if (this_hash > max_hash):
            max_hash = this_hash
    return max_hash


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        prog='Polycube Generator',
        description='Generates all polycubes (combinations of cubes) of size n.')

    parser.add_argument('n', metavar='N', type=int,
                        help='The number of cubes within each polycube')

    # Requires python >=3.9
    parser.add_argument('--cache', action=argparse.BooleanOptionalAction)
    parser.add_argument('--multicore', action=argparse.BooleanOptionalAction)
    parser.add_argument('--render', action=argparse.BooleanOptionalAction)

    init_parrelism()

    args = parser.parse_args()

    n = args.n
    use_cache = args.cache if args.cache is not None else True
    multicore = args.multicore if args.multicore is not None else False
    render = args.render if args.render is not None else False

    # Start the timer
    t1_start = perf_counter()

    all_cubes = generate_polycubes(n, use_cache=use_cache, enable_multicore=multicore)

    # Stop the timer
    t1_stop = perf_counter()

    if (render):
        render_shapes(all_cubes, "./out")

    print(f"\nFound {len(all_cubes)} unique polycubes")
    print(f"\nElapsed time: {round(t1_stop - t1_start,3)}s")
