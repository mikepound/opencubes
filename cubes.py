import numpy as np
import argparse
from time import perf_counter
from libraries.cache import get_cache, save_cache, cache_exists
from libraries.resizing import expand_cube
from libraries.packing import pack, unpack
from libraries.renderer import render_shapes
from libraries.rotation import all_rotations


def log_if_needed(n, total_n):
    if (n == total_n or n % 100 == 0):
        print(f"\rcompleted {(n / total_n) * 100:.2f}%", end="\n" if n == total_n else "")


def generate_polycubes(n: int, use_cache: bool = False) -> list[np.ndarray]:
    """
    Generates all polycubes of size n

    Generates a list of all possible configurations of n cubes, where all cubes are connected via at least one face.
    Builds each new polycube from the previous set of polycubes n-1.
    Uses an optional cache to save and load polycubes of size n-1 for efficiency.

    Parameters:
    n (int): The size of the polycubes to generate, e.g. all combinations of n=4 cubes.
    use_cahe (bool): whether to use cache files. 

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
        print(f"\nGot polycubes from cache n={n}")
    else:
        pollycubes = generate_polycubes(n-1, use_cache)

        hashes = set()
        done = 0
        print(f"\nHashing polycubes n={n}")
        for base_cube in pollycubes:
            for new_cube in expand_cube(base_cube):
                cube_hash = get_canoincal_packing(new_cube, hashes)
                hashes.add(cube_hash)
            log_if_needed(done, len(pollycubes))
            done += 1
        log_if_needed(done, len(pollycubes))

        print(f"\nGenerating polycubes from hash n={n}")
        results = []
        done = 0
        for cube_hash in hashes:
            results.append(unpack(cube_hash))
            log_if_needed(done, len(hashes))
            done += 1
        log_if_needed(done, len(hashes))

    if (use_cache and not cache_exists(n)):
        save_cache(n, results)

    return results


def get_canoincal_packing(polycube: np.ndarray, known_hashes: set[int]) -> int:
    """
    Determines if a polycube has already been seen.

    Considers all possible rotations of a cube against the existing cubes stored in memory.
    Returns True if the cube exists, or False if it is new.

    Parameters:
    polycube (np.array): 3D Numpy byte array where 1 values indicate polycube positions

    Returns:
    hash: the hash for this cube

    """
    max_hash = 0
    for cube_rotation in all_rotations(polycube):
        this_hash = pack(cube_rotation)
        if(this_hash in known_hashes):
            return this_hash
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
    parser.add_argument('--render', action=argparse.BooleanOptionalAction)

    args = parser.parse_args()

    n = args.n
    use_cache = args.cache if args.cache is not None else True
    render = args.render if args.render is not None else False

    # Start the timer
    t1_start = perf_counter()

    all_cubes = generate_polycubes(n, use_cache=use_cache)

    # Stop the timer
    t1_stop = perf_counter()

    if (render):
        render_shapes(all_cubes, "./out")

    print(f"\nFound {len(all_cubes)} unique polycubes")
    print(f"\nElapsed time: {round(t1_stop - t1_start,3)}s")
