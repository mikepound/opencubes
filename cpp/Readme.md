# C++ implementation of opencubes
- uses list representation of coordinates with ones
- hashfunction for coordinate is simple concatination of bytes
- can split problem into threads, but performance can be improoved

## usage:
```bash
./build.sh
./cubes N [NUM_THREADS]
```

## environment variable:
Cache reads can be disabled by setting USE_CACHE=0
Cache writes ... WRITE_CACHE=0

```bash
USE_CACHE=0 ./cubes N [NUM_THREADS]
# and
WRITE_CACHE=0 ./cubes N [NUM_THREADS]
```