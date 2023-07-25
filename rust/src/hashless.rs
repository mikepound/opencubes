use std::{cmp::max, time::Instant};

use crate::pcube::RawPCube;
use hashbrown::HashSet;
use indicatif::ProgressBar;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    pointlist::{array_insert, array_shift},
    polycube_reps::{CubeMapPos, Dim},
    rotations::{rot_matrix_points, to_min_rot_points, MatrixCol},
};

pub struct HashlessCubeMap<const N: usize> {
    inner: HashSet<CubeMapPos<N>>,
}

impl<const N: usize> HashlessCubeMap<N> {
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    /// helper function to not duplicate code for canonicalising polycubes
    /// and storing them in the hashset
    fn insert_map(&mut self, dim: &Dim, map: &CubeMapPos<N>, count: usize) {
        if !self.inner.contains(map) {
            let map = to_min_rot_points(map, dim, count);
            self.inner.insert(map);
        }
    }

    /// try expaning each cube into both x+1 and x-1, calculating new dimension
    /// and ensuring x is never negative
    #[inline]
    fn expand_xs(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        for (i, coord) in seed.cubes[0..count].iter().enumerate() {
            if !seed.cubes[(i + 1)..count].contains(&(coord + 1)) {
                let mut new_shape = *shape;
                let mut exp_map = *seed;

                array_insert(coord + 1, &mut exp_map.cubes[i..=count]);
                new_shape.x = max(new_shape.x, ((coord + 1) & 0x1f) as usize);
                self.insert_map(&new_shape, &exp_map, count + 1)
            }

            if coord & 0x1f != 0 {
                if !seed.cubes[0..i].contains(&(coord - 1)) {
                    let mut exp_map = *seed;
                    //faster move of top half hopefully
                    array_shift(&mut exp_map.cubes[i..=count]);
                    array_insert(coord - 1, &mut exp_map.cubes[0..=i]);
                    self.insert_map(shape, &exp_map, count + 1)
                }
            } else {
                let mut new_shape = *shape;
                new_shape.x += 1;
                let mut exp_map = *seed;
                for i in 0..count {
                    exp_map.cubes[i] += 1;
                }
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(*coord, &mut exp_map.cubes[0..=i]);
                self.insert_map(&new_shape, &exp_map, count + 1)
            }
        }
    }

    /// Try expanding each cube into both y+1 and y-1, calculating new dimension
    /// and ensuring y is never negative
    #[inline]
    fn expand_ys(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        for (i, coord) in seed.cubes[..count].iter().enumerate() {
            let y_plus = coord + (1 << 5);
            let y_minus = coord - (1 << 5);

            let mut new_map = *seed;
            let mut new_shape = *shape;

            if !seed.cubes[(i + 1)..count].contains(&y_plus) {
                let mut new_shape = *shape;
                let mut exp_map = *seed;
                array_insert(y_plus, &mut exp_map.cubes[i..=count]);
                new_shape.y = max(new_shape.y, (((coord >> 5) + 1) & 0x1f) as usize);
                self.insert_map(&new_shape, &exp_map, count + 1)
            }

            // Determine the new shape and the coordinate at which the next cube
            // will be inserted.
            let insert_coord = if (coord >> 5) & 0x1f != 0 {
                if !seed.cubes[0..i].contains(&y_minus) {
                    y_minus
                } else {
                    continue;
                }
            } else {
                new_shape.y += 1;
                for i in 0..count {
                    new_map.cubes[i] += 1 << 5;
                }
                *coord
            };

            array_shift(&mut new_map.cubes[i..=count]);
            array_insert(insert_coord, &mut new_map.cubes[0..=i]);
            self.insert_map(&new_shape, &new_map, count + 1)
        }
    }

    /// try expaning each cube into both z+1 and z-1, calculating new dimension
    /// and ensuring z is never negative
    #[inline]
    fn expand_zs(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        for (i, coord) in seed.cubes[0..count].iter().enumerate() {
            if !seed.cubes[(i + 1)..count].contains(&(coord + (1 << 10))) {
                let mut new_shape = *shape;
                let mut exp_map = *seed;
                array_insert(coord + (1 << 10), &mut exp_map.cubes[i..=count]);
                new_shape.z = max(new_shape.z, (((coord >> 10) + 1) & 0x1f) as usize);
                self.insert_map(&new_shape, &exp_map, count + 1)
            }

            if (coord >> 10) & 0x1f != 0 {
                if !seed.cubes[0..i].contains(&(coord - (1 << 10))) {
                    let mut exp_map = *seed;
                    //faster move of top half hopefully
                    array_shift(&mut exp_map.cubes[i..=count]);
                    array_insert(coord - (1 << 10), &mut exp_map.cubes[0..=i]);
                    self.insert_map(shape, &exp_map, count + 1)
                }
            } else {
                let mut new_shape = *shape;
                new_shape.z += 1;
                let mut exp_map = *seed;
                for i in 0..count {
                    exp_map.cubes[i] += 1 << 10;
                }
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(*coord, &mut exp_map.cubes[0..=i]);
                self.insert_map(&new_shape, &exp_map, count + 1)
            }
        }
    }

    /// reduce number of expansions needing to be performed based on
    /// X >= Y >= Z constraint on Dim
    #[inline]
    fn do_cube_expansion(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        if shape.y < shape.x {
            self.expand_ys(seed, shape, count);
        }
        if shape.z < shape.y {
            self.expand_zs(seed, shape, count);
        }
        self.expand_xs(seed, shape, count);
    }

    /// perform the cube expansion for a given polycube
    /// if perform extra expansions for cases where the dimensions are equal as
    /// square sides may miss poly cubes otherwise
    #[inline]
    fn expand_cube_map(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        if shape.x == shape.y && shape.x > 0 {
            let rotz = rot_matrix_points(
                seed,
                shape,
                count,
                MatrixCol::YN,
                MatrixCol::XN,
                MatrixCol::ZN,
                1025,
            );
            self.do_cube_expansion(&rotz, shape, count);
        }

        if shape.y == shape.z && shape.y > 0 {
            let rotx = rot_matrix_points(
                seed,
                shape,
                count,
                MatrixCol::XN,
                MatrixCol::ZP,
                MatrixCol::YP,
                1025,
            );
            self.do_cube_expansion(&rotx, shape, count);
        }
        if shape.x == shape.z && shape.x > 0 {
            let roty = rot_matrix_points(
                seed,
                shape,
                count,
                MatrixCol::ZP,
                MatrixCol::YP,
                MatrixCol::XN,
                1025,
            );
            self.do_cube_expansion(&roty, shape, count);
        }

        self.do_cube_expansion(seed, shape, count);
    }

    fn enumerate_canonical_children(seed: &CubeMapPos<N>, count: usize, target: usize) -> usize {
        let mut map = Self::new();
        let shape = seed.extrapolate_dim();
        map.expand_cube_map(seed, &shape, count);

        map.inner
            .retain(|child| child.is_canonical_root(count, seed));

        if count + 1 == target {
            map.inner.len()
        } else {
            map.inner
                .iter()
                .map(|child| Self::enumerate_canonical_children(child, count + 1, target))
                .sum()
        }
    }
}

/// run pointlist based generation algorithm
pub fn gen_polycubes(
    n: usize,
    parallel: bool,
    current: Vec<RawPCube>,
    calculate_from: usize,
    bar: &ProgressBar,
) -> usize {
    let t1_start = Instant::now();

    let seed_count = current.len();
    bar.set_length(seed_count as u64);
    bar.set_message(format!(
        "seed subsets expanded for N = {}...",
        calculate_from - 1
    ));

    let process = |seed: CubeMapPos<32>| {
        let children = HashlessCubeMap::enumerate_canonical_children(&seed, calculate_from - 1, n);
        bar.set_message(format!(
            "seed subsets expanded for N = {}...",
            calculate_from - 1,
        ));
        bar.inc(1);
        children
    };

    //convert input vector of NaivePolyCubes and convert them to
    let count: usize = if parallel {
        current
            .par_iter()
            .map(|seed| seed.into())
            .map(process)
            .sum()
    } else {
        current.iter().map(|seed| seed.into()).map(process).sum()
    };
    let time = t1_start.elapsed().as_micros();
    bar.set_message(format!(
        "Found {} unique expansions (N = {n}) in  {}.{:06}s",
        count,
        time / 1000000,
        time % 1000000
    ));

    bar.finish();
    count
    //count_polycubes(&seeds);
}
