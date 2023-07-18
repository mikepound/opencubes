## Running the code
With python installed, you can run the code like this:

`python cubes.py --cache n`

Where n is the number of cubes you'd like to calculate. If you specify `--cache` then the program will attempt to load .npy files that hold all the pre-computed cubes for n-1 and then n. If you specify `--no-cache` then everything is calcuated from scratch, and no cache files are stored.

## Testing your changes.
If you are contributing to the python version of this project, you can find some unit tests in the tests folder.
these can be run with:

`python -m unittest`

these tests are not complete or rigerous but they might help spot obvious errors in any changes you make.

## How the code works
The code includes some doc strings to help you understand what it does, but in short it operates a bit like this (oversimplified!):

To generate all combinations of n cubes, we first calculate all possible n-1 shapes based on the same algorithm. We begin by taking all of the n-1 shape, and for each of these add new cubes in any possible free locations. For each of these potential new shapes, we test each rotation of this shape to see if it's been seen before. Entirely new shapes are added to a set of all shapes, to check future candidates.

## Pre-computed cache files
You can download the cache files for n=3 to n=12 from [here](https://drive.google.com/drive/folders/1Ls3gJCrNQ17yg1IhrIav70zLHl858Fl4?usp=drive_link). If you manage to calculate any more sets, please feel free to save them as an npy file and I'll upload them!
