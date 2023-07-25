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

macro_rules! define_expand_fn {
    ($name:ident, $shift:literal, $dim:ident, $dim_str:literal) => {
        /// Try expanding each cube into
        #[doc = $dim_str]
        /// plus one and
        #[doc = $dim_str]
        /// minus one , calculating new dimension and ensuring
        #[doc = $dim_str]
        /// is never negative
        #[inline(always)]
        fn $name(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
            for (i, coord) in seed.cubes[0..count].iter().enumerate() {
                let plus = coord + (1 << $shift);
                let minus = coord - (1 << $shift);

                // Check if we can insert a new cube at $dim + 1
                if !seed.cubes[(i + 1)..count].contains(&plus) {
                    let mut new_shape = *shape;
                    let mut exp_map = *seed;

                    array_insert(plus, &mut exp_map.cubes[i..=count]);
                    new_shape.$dim = max(new_shape.$dim, (((coord >> $shift) + 1) & 0x1f) as usize);
                    self.insert_map(&new_shape, &exp_map, count + 1)
                }

                let mut new_map = *seed;
                let mut new_shape = *shape;

                // If the coord is out of bounds for $dim, shift everything
                // over and insert a new cube at the out-of-bounds position.
                // If it is in bounds, check if the $dim - 1 value is already
                // set.
                // NOTE(datdenkikniet): ^^ I deduced this. Is it correct?
                let insert_coord = if (coord >> $shift) & 0x1f != 0 {
                    if !seed.cubes[0..i].contains(&minus) {
                        minus
                    } else {
                        continue;
                    }
                } else {
                    new_shape.$dim += 1;
                    for i in 0..count {
                        new_map.cubes[i] += 1 << $shift;
                    }
                    *coord
                };

                array_shift(&mut new_map.cubes[i..=count]);
                array_insert(insert_coord, &mut new_map.cubes[0..=i]);
                self.insert_map(&new_shape, &new_map, count + 1)
            }
        }
    };
}

impl<const N: usize> HashlessCubeMap<N> {
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    define_expand_fn!(expand_xs, 0, x, "x");
    define_expand_fn!(expand_ys, 5, y, "y");
    define_expand_fn!(expand_zs, 10, z, "z");

    /// helper function to not duplicate code for canonicalising polycubes
    /// and storing them in the hashset
    fn insert_map(&mut self, dim: &Dim, map: &CubeMapPos<N>, count: usize) {
        if !self.inner.contains(map) {
            let map = to_min_rot_points(map, dim, count);
            self.inner.insert(map);
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
        use MatrixCol::*;

        if shape.x == shape.y && shape.x > 0 {
            let rotz = rot_matrix_points(seed, shape, count, YN, XN, ZN, 1025);
            self.do_cube_expansion(&rotz, shape, count);
        }

        if shape.y == shape.z && shape.y > 0 {
            let rotx = rot_matrix_points(seed, shape, count, XN, ZP, YP, 1025);
            self.do_cube_expansion(&rotx, shape, count);
        }
        if shape.x == shape.z && shape.x > 0 {
            let roty = rot_matrix_points(seed, shape, count, ZP, YP, XN, 1025);
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
