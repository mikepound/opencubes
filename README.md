# Polycubes
This code is associated with the Computerphile video on generating polycubes. The original repository may be found [here](https://github.com/mikepound/cubes). That version is unchanged from my original video, so that those watching for the first time can find and use the original code, and make improvements to it themselves. This repository is for those looking to contribute to a faster and better optimised version, driven by improvements from Computerphile viewers!

## Introduction
A polycube is a set of cubes in any configuration in which all cubes are orthogonally connected - share a face. This code calculates all the variations of 3D polycubes for any size (time permitting!). 

![5cubes](https://github.com/mikepound/cubes/assets/9349459/4fe60d01-c197-4cb3-b298-1dbae8517a74)


## How the code works
The code includes some doc strings to help you understand what it does, but in short it operates a bit like this (oversimplified!):

To generate all combinations of n cubes, we first calculate all possible n-1 shapes based on the same algorithm. We begin by taking all of the n-1 shape, and for each of these add new cubes in any possible free locations. For each of these potential new shapes, we test each rotation of this shape to see if it's been seen before. Entirely new shapes are added to a set of all shapes, to check future candidates.

## Running the code
With python installed, you can run the code like this:

`python cubes.py --cache n`

Where n is the number of cubes you'd like to calculate. If you specify `--cache` then the program will attempt to load .npy files that hold all the pre-computed cubes for n-1 and then n. If you specify `--no-cache` then everything is calcuated from scratch, and no cache files are stored.

## Testing your changes.
If you are contributing to the python version of this project, you can find some unit tests in the tests folder.
these can be run with "python -m unittest". these tests are not complete or rigerous but they might help spot obvious errors in any changes you make.

## Pre-computed cache files
You can download the cache files for n=3 to n=12 from [here](https://drive.google.com/drive/folders/1Ls3gJCrNQ17yg1IhrIav70zLHl858Fl4?usp=drive_link). If you manage to calculate any more sets, please feel free to save them as an npy file and I'll upload them!

## Improving the code
This repo already has some improvements included, and will happily accept more via pull request.
Some things you might think about:
- C++ and Rust implementations are currently in development, if you would like to contribute to these look at the pull requests (or of course feel free to start you own).
- The main limiting factor at this time seems to be memory usage, at n=14+ you need hundereds of GB's just to store the cubes, so keeping them all in main memory gets dificult.
- Distributing the computation across many systems would allow us to scale horizontally rather than vertically, but it opens questions of how to do so without each system having a full copy of all the cubes, and how to manage the large quantities of data.
- Calculating 24 rotations of a cube is slow, the only way to avoid this would be to come up with some rotationally invariant way of comparing cubes. I've not thought of one yet!

## Contributing!
This version welcomes contributors!

## References
- [Wikipedia article](https://en.wikipedia.org/wiki/Polycube)
- [This repository](https://github.com/noelle-crawfish/Enumerating-Polycubes) was a source of inspiration, and a great description of some possible ways to solve this.
- [There may be better ways](https://www.sciencedirect.com/science/article/pii/S0012365X0900082X) to count these, but I've not explored in much detail.
- [Kevin Gong's](http://kevingong.com/Polyominoes/Enumeration.html) webpage on enumerating all shapes up to n=16.
