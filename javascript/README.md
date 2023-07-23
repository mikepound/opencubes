# Javascript implementation

## Running the code

```
node cubes.js 10
 > Found 346543 polycubes of size 10
 > time: 20.347s
```

This is currently an order of magnitude faster than python and an order of magnitude slower than c++.

## New Canonical Encoding

Given a root cube and an orientation, we can encode a cube by its adjacency graph.  I'll illustrate my method by encoding this polycube with 5 cubes.

```
         +--+
        /  /|
    +--+--+--+
   /  /  /  / |
  +--+--+--+  |
  |  |  |  | /|
  +--+--+--+  |
        |  | /
        +--+
```

We choose a cube (cube #0) and the orientation [Left Right Up Down Forward Back].  Record the adjacent cubes to cube #0 as six bits:

```
         +--+
        /  /|
    +--+--+--+
   /  /  /  / |
  +--+--+--+  |
  |#0|  |  | /|
  +--+--+--+  |
        |  | /
        +--+

  Cube 0
  LRUDFB
  010000
```

We add any adjacent cubes to our cube order.  We have marked cube #1.  We can now repeat the process for cube #1:

```
         +--+
        /  /|
    +--+--+--+
   /  /  /  / |
  +--+--+--+  |
  |#0|#1|  | /|
  +--+--+--+  |
        |  | /
        +--+

  Cube 0  Cube 1
  LRUDFB  LRUDFB
  010000  110001
```

There are two new adjacent cubes.  They are added to our list of cubes in the same order as our orientation (in this case, first Left, then Right, Up, Down, Forward and Back.  This would be different if we picked a different orientation).  We now repeat the process for cube #2:

```
         +--+
        /#3/|
    +--+--+--+
   /  /  /  / |
  +--+--+--+  |
  |#0|#1|#2| /|
  +--+--+--+  |
        |  | /
        +--+

  Cube 0  Cube 1  Cube 2
  LRUDFB  LRUDFB  LRUDFB
  010000  110001  100100
```

At this point we have completely ordered our cubes.  Any additional bits would be redundant.  We finish our encoding with six 0 bits.

```
         +--+
        /#3/|
    +--+--+--+         Representation choosing cube #0 and LRUDFB:
   /  /  /  / |
  +--+--+--+  |        010000  110001  100100  000000
  |#0|#1|#2| /|
  +--+--+--+  |
        |#4| /
        +--+
```

Define the canonical representation of a cube to be the maximum represention over all choices of first cube and orientation.

```
         +--+
        /#3/|
    +--+--+--+         Canonical representation:
   /  /  /  / |
  +--+--+--+  |        111000  010010  000000
  |#2|#0|#1| /|
  +--+--+--+  |
        |#4| /
        +--+
```

This was encoded by choosing the middle cube to be cube #0 - this is the only cube that can be encoded with three ones at the beginning of its adjacency graph, so we do not need to consider any other cubes as root. The representation was maxed when we chose the orientation RLBFDU.

When we parse this representation, we do not know the orientation it was encoded with, so we will get the same cube in (possibly) a different orientation.

```
Representation: 111000 010010 000000
Parsing orientation:    LRUDFB

  Initial    After parsing    After parsing
  state:     6 bits:          12 bits:
                  ___             ___
                 /  /|           /  /|
      ___      _+--+ |__       _+--+ |__
     /  /|    / |#3|/  /|     / |#3|/  /|
    +--+ |   +--+--+--+ |    +--+--+--+ |
    |#0|/    |#1|#0|#2|/    /  /|#0|#2|/
    +--+     +--+--+--+    |⎺⎺| +--+--+
                           |#4|/
                            ⎺⎺
```

Note that this canonical representation gives a unique ordering of the cubes, and the cubes are in increasing distance from the root cube.

So we can take away the last cube, leaving a connected polycube.  This gives a well-defined manner of assigning a polycube with n-1 cubes to a given polycube with n cubes.

This allows us to not use a hash table and parallelize computation as in issue #11.
  1.  Start with a cube p, and extend p by one cube to p+1
  2.  Find the canonical representation of p+1, and remove the last cube to create p+1-1
  3.  If p+1-1 is the same polycube as p (in some orientation), save p+1

I know other people have already implemented ways of getting rid of a hash table, or splitting computation computation by the dimensions of the polycube.  (see issue #27 and PR #26 and #28).  I haven't had the time to fully read through everything that has been done, but hopefully this is a somewhat novel way of looking at the problem.  It's been a lot of fun coding a proof of concept even if I know a javascript implementation isn't going to go anywhere ;)

## Notes of the code

It didn't turn out pretty, there's a lot of mutated state.  I wanted to prioritize making a PR over cleaning up the code.


