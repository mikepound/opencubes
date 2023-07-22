//! This module implements an iterator that provides all N + 1 expansions
//! for a polycube of N.

use super::NaivePolyCube;

type Sides = std::array::IntoIter<(usize, usize, usize), 6>;

#[derive(Clone)]
struct ExpansionIterator {
    dim_1: usize,
    dim_2: usize,
    dim_3: usize,
    iterating_cube: Option<Sides>,
    padded_cube: NaivePolyCube,
    done: bool,
}

impl ExpansionIterator {
    /// Move to the next possibly-occupied box
    ///
    /// Returns `true` if we're done iterating
    fn go_to_next(&mut self) -> bool {
        let (d1, d2, d3) = self.padded_cube.dims();
        if self.dim_1 == d1 - 1 && self.dim_2 == d2 - 1 {
            self.dim_1 = 1;
            self.dim_2 = 1;
            self.dim_3 += 1;
        } else if self.dim_1 == d1 - 1 {
            self.dim_1 = 1;
            self.dim_2 += 1;
        } else {
            self.dim_1 += 1;
        }

        self.dim_3 == d3 - 1
    }

    fn faces(d1: usize, d2: usize, d3: usize) -> std::array::IntoIter<(usize, usize, usize), 6> {
        [
            (d1 + 1, d2, d3),
            (d1 - 1, d2, d3),
            (d1, d2 + 1, d3),
            (d1, d2 - 1, d3),
            (d1, d2, d3 + 1),
            (d1, d2, d3 - 1),
        ]
        .into_iter()
    }
}

impl Iterator for ExpansionIterator {
    type Item = NaivePolyCube;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done && self.iterating_cube.is_none() {
                return None;
            }

            if self.iterating_cube.is_none() {
                loop {
                    let (d1, d2, d3) = (self.dim_1, self.dim_2, self.dim_3);

                    if self.go_to_next() {
                        self.done = true;
                        return None;
                    }

                    if self.padded_cube.is_set(d1, d2, d3) {
                        self.iterating_cube = Some(Self::faces(d1, d2, d3));
                        break;
                    }
                }
            }

            if let Some(ref mut sides) = self.iterating_cube {
                let (d1, d2, d3) = sides.next().unwrap();

                if sides.len() == 0 {
                    self.iterating_cube.take();
                }

                // If the cube is already set, skip this face
                if self.padded_cube.is_set(d1, d2, d3) {
                    continue;
                }

                let mut next_cube = self.padded_cube.clone();
                next_cube.set(d1, d2, d3).unwrap();
                return Some(next_cube);
            } else {
                return None;
            }
        }
    }
}

impl NaivePolyCube {
    pub fn expand(&self) -> impl Iterator<Item = NaivePolyCube> + Clone {
        ExpansionIterator {
            dim_1: 1,
            dim_2: 1,
            dim_3: 1,
            iterating_cube: None,
            padded_cube: self.pad_one(),
            done: false,
        }
    }
}
