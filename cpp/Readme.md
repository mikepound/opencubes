# C++ implementation of opencubes
- uses list representation of coordinates with ones
- hashfunction for coordinate is simple concatination of bytes
- can split problem into threads, but performance of merging results is poor.
- no caching implemented yet
- usage of vector should be unnecessary because all the information is in the set

## usage:
```bash
./build.sh
./cubes N [NUM_THREADS]
```