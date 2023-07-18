# Polycubes
This code is associated with the Computerphile video on generating polycubes. The original repository may be found [here](https://github.com/mikepound/cubes). That version is unchanged from my original video, so that those watching for the first time can find and use the original code, and make improvements to it themselves. This repository is for those looking to contribute to a faster and better optimised version, driven by improvements from Computerphile viewers!

## Introduction
A polycube is a set of cubes in any configuration in which all cubes are orthogonally connected - share a face. This code calculates all the variations of 3D polycubes for any size (time permitting!). 

![5cubes](https://github.com/mikepound/cubes/assets/9349459/4fe60d01-c197-4cb3-b298-1dbae8517a74)

## Contents
This repository contains 3 solutions written in 3 languages Python, C++, and Rust.
each sub folder contains a README with instructions on how to run them.

## Improving the code
This repo already has some improvements included, and will happily accept more via pull request.
Some things you might think about:
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
