from libraries.cache import get_cache_raw

def get_test_data():
    return get_cache_raw('./tests/test_data.npy')