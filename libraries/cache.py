import os
import numpy as np

cache_path_fstring = "cubes_{0}.npy"


def cache_exists(n: int) -> bool:
    """
    Checks if a cache file exists with the standard name for a given batch size

    Parameters:
    n (int): the size of polycube to search for

    Returns:
    bool: whether that cache exists

    """
    cache_path = cache_path_fstring.format(n)
    return os.path.exists(cache_path)


def get_cache_raw(cache_path: str) -> list[np.ndarray]:
    """
    Loads a Cache File for a given pathname

    Parameters:
    cache_path (str): the file location to look for the cache file

    Returns:
    list[np.ndarray]: the list of polycubes from the cache

    """
    if os.path.exists(cache_path):

        polycubes = np.load(cache_path, allow_pickle=True)

        return polycubes
    else:
        return None


def get_cache(n: int) -> np.ndarray:
    """
    Loads a Cache File for a given size of polycube

    Parameters:
    n (int): the size of polycube to load the cache of

    Returns:
    list[np.ndarray]: the list of polycubes of that size from the cache

    """
    cache_path = cache_path_fstring.format(n)
    print(f"\rLoading polycubes n={n} from cache: ", end="")
    polycubes = get_cache_raw(cache_path)
    print(f"{len(polycubes)} shapes")
    return polycubes


def save_cache_raw(cache_path: str, polycubes: list[np.ndarray]) -> None:
    """
    Saves a Cache File to a file at a given pathname

    Parameters:
    cache_path (str): the file location to sabe the cache file
    polycubes (list[np.ndarray]): the polycubes to be cached
    """
    np.save(cache_path, np.array(polycubes, dtype=object), allow_pickle=True)


def save_cache(n: int, polycubes: np.ndarray) -> None:
    """
    Saves a Cache File for a given polycube size

    Parameters:
    n (int): the size of the polycubes to be cached
    polycubes (list[np.ndarray]): the polycubes to be cached
    """
    cache_path = cache_path_fstring.format(n)
    save_cache_raw(cache_path, polycubes)
    print(f"Wrote file for polycubes n={n}")
