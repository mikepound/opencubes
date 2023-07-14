# C++ implementation of opencubes
- uses list representation of coordinates with ones
- hashfunction for coordinate is simple concatination of bytes
- can split problem into threads, but performance can be improoved

## usage:
```bash
./build.sh
./cubes N [NUM_THREADS]
```