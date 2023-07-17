# C++ implementation of opencubes
- uses list representation of coordinates with ones
- hashfunction for coordinate is simple concatination of bytes
- can split problem into threads, but performance can be improoved

## usage:
```bash
./cubes -n N
```
options:
```
-t    --threads
the number of threads to use while generating
This parameter is optional. The default value is '1'.

-c    --use_cache
whether to load cache files
This parameter is optional. The default value is '0'.

-w    --write_cache
wheather to save cache files
This parameter is optional. The default value is '0'.
```

## building (cmake)
```bash
mkdir build && cd build
cmake ..
make
```