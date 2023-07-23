use std::{cmp::max, time::Instant};

use hashbrown::HashSet;
use indicatif::ProgressBar;
use crate::pcube::RawPCube;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    pointlist::{array_insert, array_shift},
    polycube_reps::{CubeMapPos, Dim},
    rotations::{rot_matrix_points, to_min_rot_points, MatrixCol},
};

/// helper function to not duplicate code for canonicalising polycubes
/// and storing them in the hashset
fn insert_map(store: &mut HashSet<CubeMapPos<32>>, dim: &Dim, map: &CubeMapPos<32>, count: usize) {
    if !store.contains(map) {
        let map = to_min_rot_points(map, dim, count);
        store.insert(map);
    }
}

/// try expaning each cube into both x+1 and x-1, calculating new dimension
/// and ensuring x is never negative
#[inline]
fn expand_xs(dst: &mut HashSet<CubeMapPos<32>>, seed: &CubeMapPos<32>, shape: &Dim, count: usize) {
    for (i, coord) in seed.cubes[0..count].iter().enumerate() {
        if !seed.cubes[(i + 1)..count].contains(&(coord + 1)) {
            let mut new_shape = *shape;
            let mut exp_map = *seed;

            array_insert(coord + 1, &mut exp_map.cubes[i..=count]);
            new_shape.x = max(new_shape.x, ((coord + 1) & 0x1f) as usize);
            insert_map(dst, &new_shape, &exp_map, count + 1)
        }
        if coord & 0x1f != 0 {
            if !seed.cubes[0..i].contains(&(coord - 1)) {
                let mut exp_map = *seed;
                //faster move of top half hopefully
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(coord - 1, &mut exp_map.cubes[0..=i]);
                insert_map(dst, shape, &exp_map, count + 1)
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
            insert_map(dst, &new_shape, &exp_map, count + 1)
        }
    }
}

/// try expaning each cube into both y+1 and y-1, calculating new dimension
/// and ensuring y is never negative
#[inline]
fn expand_ys(dst: &mut HashSet<CubeMapPos<32>>, seed: &CubeMapPos<32>, shape: &Dim, count: usize) {
    for (i, coord) in seed.cubes[0..count].iter().enumerate() {
        if !seed.cubes[(i + 1)..count].contains(&(coord + (1 << 5))) {
            let mut new_shape = *shape;
            let mut exp_map = *seed;
            array_insert(coord + (1 << 5), &mut exp_map.cubes[i..=count]);
            new_shape.y = max(new_shape.y, (((coord >> 5) + 1) & 0x1f) as usize);
            insert_map(dst, &new_shape, &exp_map, count + 1)
        }
        if (coord >> 5) & 0x1f != 0 {
            if !seed.cubes[0..i].contains(&(coord - (1 << 5))) {
                let mut exp_map = *seed;
                //faster move of top half hopefully
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(coord - (1 << 5), &mut exp_map.cubes[0..=i]);
                insert_map(dst, shape, &exp_map, count + 1)
            }
        } else {
            let mut new_shape = *shape;
            new_shape.y += 1;
            let mut exp_map = *seed;
            for i in 0..count {
                exp_map.cubes[i] += 1 << 5;
            }
            array_shift(&mut exp_map.cubes[i..=count]);
            array_insert(*coord, &mut exp_map.cubes[0..=i]);
            insert_map(dst, &new_shape, &exp_map, count + 1)
        }
    }
}

/// try expaning each cube into both z+1 and z-1, calculating new dimension
/// and ensuring z is never negative
#[inline]
fn expand_zs(dst: &mut HashSet<CubeMapPos<32>>, seed: &CubeMapPos<32>, shape: &Dim, count: usize) {
    for (i, coord) in seed.cubes[0..count].iter().enumerate() {
        if !seed.cubes[(i + 1)..count].contains(&(coord + (1 << 10))) {
            let mut new_shape = *shape;
            let mut exp_map = *seed;
            array_insert(coord + (1 << 10), &mut exp_map.cubes[i..=count]);
            new_shape.z = max(new_shape.z, (((coord >> 10) + 1) & 0x1f) as usize);
            insert_map(dst, &new_shape, &exp_map, count + 1)
        }
        if (coord >> 10) & 0x1f != 0 {
            if !seed.cubes[0..i].contains(&(coord - (1 << 10))) {
                let mut exp_map = *seed;
                //faster move of top half hopefully
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(coord - (1 << 10), &mut exp_map.cubes[0..=i]);
                insert_map(dst, shape, &exp_map, count + 1)
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
            insert_map(dst, &new_shape, &exp_map, count + 1)
        }
    }
}

/// reduce number of expansions needing to be performed based on
/// X >= Y >= Z constraint on Dim
#[inline]
fn do_cube_expansion(
    dst: &mut HashSet<CubeMapPos<32>>,
    seed: &CubeMapPos<32>,
    shape: &Dim,
    count: usize,
) {
    if shape.y < shape.x {
        expand_ys(dst, seed, shape, count);
    }
    if shape.z < shape.y {
        expand_zs(dst, seed, shape, count);
    }
    expand_xs(dst, seed, shape, count);
}

/// perform the cube expansion for a given polycube
/// if perform extra expansions for cases where the dimensions are equal as
/// square sides may miss poly cubes otherwise
#[inline]
fn expand_cube_map(
    dst: &mut HashSet<CubeMapPos<32>>,
    seed: &CubeMapPos<32>,
    shape: &Dim,
    count: usize,
) {
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
        do_cube_expansion(dst, &rotz, shape, count);
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
        do_cube_expansion(dst, &rotx, shape, count);
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
        do_cube_expansion(dst, &roty, shape, count);
    }
    do_cube_expansion(dst, seed, shape, count);
}

fn enumerate_canonical_children(seed: &CubeMapPos<32>, count: usize, target: usize) -> usize {
    let mut children = HashSet::new();
    children.clear();
    let shape = seed.extrapolate_dim();
    expand_cube_map(&mut children, seed, &shape, count);
    children.retain(|child| child.is_canonical_root(count, seed));
    if count + 1 == target {
        children.len()
    } else {
        children
            .iter()
            .map(|child| enumerate_canonical_children(child, count + 1, target))
            .sum()
    }
}

/// run pointlist based generation algorithm
pub fn gen_polycubes(
    n: usize,
    parallel: bool,
    current: Vec<RawPCube>,
    calculate_from: usize,
    bar: &ProgressBar
) -> usize {
    let t1_start = Instant::now();

    let seed_count = current.len();
    bar.set_length(seed_count as u64);
    bar.set_message(format!(
        "seed subsets expanded for N = {}...",
        calculate_from - 1
    ));

    let process = |seed| {
        let children = enumerate_canonical_children(&seed, calculate_from - 1, n);
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
