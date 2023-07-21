use std::{
    cell::RefCell,
    cmp::{max, min},
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use hashbrown::HashSet;
use opencubes::pcube::RawPCube;
use parking_lot::RwLock;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    make_bar,
    pointlist::{array_insert, array_shift},
    polycube_reps::{CubeMapPos, Dim},
    rotations::{map_coord, rot_matrix_points, to_min_rot_points, MatrixCol},
    Compression,
};

fn is_continuous(polycube: &[u16]) -> bool {
    let start = polycube[0];
    let mut visited = HashSet::new();
    let mut to_explore = HashSet::new();
    to_explore.insert(start);
    while let Some(p) = to_explore.iter().next() {
        let p = *p;
        to_explore.remove(&p);
        visited.insert(p);
        if p & 0x1f != 0 && polycube.contains(&(p - 1)) && !visited.contains(&(p - 1)) {
            to_explore.insert(p - 1);
        }
        if p & 0x1f != 0x1f && polycube.contains(&(p + 1)) && !visited.contains(&(p + 1)) {
            to_explore.insert(p + 1);
        }
        if (p >> 5) & 0x1f != 0
            && polycube.contains(&(p - (1 << 5)))
            && !visited.contains(&(p - (1 << 5)))
        {
            to_explore.insert(p - (1 << 5));
        }
        if (p >> 5) & 0x1f != 0x1f
            && polycube.contains(&(p + (1 << 5)))
            && !visited.contains(&(p + (1 << 5)))
        {
            to_explore.insert(p + (1 << 5));
        }
        if (p >> 10) & 0x1f != 0
            && polycube.contains(&(p - (1 << 10)))
            && !visited.contains(&(p - (1 << 10)))
        {
            to_explore.insert(p - (1 << 10));
        }
        if (p >> 10) & 0x1f != 0x1f
            && polycube.contains(&(p + (1 << 10)))
            && !visited.contains(&(p + (1 << 10)))
        {
            to_explore.insert(p + (1 << 10));
        }
    }
    visited.len() == polycube.len()
}

fn renormalize(exp: &CubeMapPos, dim: &Dim, count: usize) -> CubeMapPos {
    let mut dst = CubeMapPos { cubes: [0; 16] };
    let x = dim.x;
    let y = dim.y;
    let z = dim.z;
    let (x_col, y_col, z_col) = if x >= y && y >= z {
        (MatrixCol::XP, MatrixCol::YP, MatrixCol::ZP)
    } else if x >= z && z >= y {
        (MatrixCol::XP, MatrixCol::ZP, MatrixCol::YN)
    } else if y >= x && x >= z {
        (MatrixCol::YP, MatrixCol::XP, MatrixCol::ZN)
    } else if y >= z && z >= x {
        (MatrixCol::YP, MatrixCol::ZP, MatrixCol::XP)
    } else if z >= x && x >= y {
        (MatrixCol::ZN, MatrixCol::XP, MatrixCol::YN)
    } else if z >= y && y >= x {
        (MatrixCol::ZN, MatrixCol::YN, MatrixCol::XP)
    } else {
        panic!("imposible dimension of shape {:?}", dim)
    };
    for (i, d) in exp.cubes[0..count].iter().enumerate() {
        let dx = d & 0x1f;
        let dy = (d >> 5) & 0x1f;
        let dz = (d >> 10) & 0x1f;
        let cx = map_coord(dx, dy, dz, &dim, x_col);
        let cy = map_coord(dx, dy, dz, &dim, y_col);
        let cz = map_coord(dx, dy, dz, &dim, z_col);
        let pack = ((cz << 10) | (cy << 5) | cx) as u16;
        dst.cubes[i] = pack;
    }
    //dst.cubes.sort();
    dst
}

fn remove_cube(exp: &CubeMapPos, point: usize, count: usize) -> (CubeMapPos, Dim) {
    let mut min_corner = Dim {
        x: 0x1f,
        y: 0x1f,
        z: 0x1f,
    };
    let mut max_corner = Dim { x: 0, y: 0, z: 0 };
    let mut root_candidate = CubeMapPos { cubes: [0; 16] };
    let mut candidate_ptr = 0;
    for i in 0..=count {
        if i != point {
            let pos = exp.cubes[i];
            let x = pos as usize & 0x1f;
            let y = (pos as usize >> 5) & 0x1f;
            let z = (pos as usize >> 10) & 0x1f;
            min_corner.x = min(min_corner.x, x);
            min_corner.y = min(min_corner.y, y);
            min_corner.z = min(min_corner.z, z);
            max_corner.x = max(max_corner.x, x);
            max_corner.y = max(max_corner.y, y);
            max_corner.z = max(max_corner.z, z);
            root_candidate.cubes[candidate_ptr] = exp.cubes[i];
            candidate_ptr += 1;
        }
    }
    // println!("u{:?}", root_candidate.cubes);
    let offset = (min_corner.z << 10) | (min_corner.y << 5) | min_corner.x;
    for i in 0..count {
        root_candidate.cubes[i] -= offset as u16;
    }
    // println!("w{:?}", root_candidate.cubes);
    max_corner.x = max_corner.x - min_corner.x;
    max_corner.y = max_corner.y - min_corner.y;
    max_corner.z = max_corner.z - min_corner.z;
    (root_candidate, max_corner)
}

fn is_canonical_root(exp: &CubeMapPos, count: usize, seed: &CubeMapPos) -> bool {
    let mut seed_found = false;
    for sub_cube_id in 0..=count {
        let (mut root_candidate, dim) = remove_cube(exp, sub_cube_id, count);
        // if dim.x == 1 && dim.y == 1 && dim.z == 1 {
        //     println!("111 {:?} {}", root_candidate, count);
        // }
        if dim.x < dim.y || dim.y < dim.z || dim.x < dim.z {
            root_candidate = renormalize(&root_candidate, &dim, count);
            // println!("a");
            // continue;
        }
        if !is_continuous(&root_candidate.cubes[0..count]) {
            // println!("b");
            continue;
        }
        root_candidate.cubes[0..count].sort_unstable();
        // println!("c");
        let mrp = to_min_rot_points(&root_candidate, &root_candidate.extrapolate_dim(), count);
        if &mrp < seed {
            return false;
        }
        // println!("mrp {:?}", mrp);
        if &mrp == seed {
            seed_found = true;
            // println!("seedy");
        }
    }
    if !seed_found {
        panic!("seedless {}\n{:?}\n{:?}", count, exp, seed);
    }
    // println!("{:?} root of {:?}", exp, seed);
    true
}

/// helper function to not duplicate code for canonicalising polycubes
/// and storing them in the hashset
fn insert_map(store: &mut HashSet<CubeMapPos>, dim: &Dim, map: &CubeMapPos, count: usize) {
    // if map.extrapolate_count() != count {
    //     panic!("count missmatch {} {}", map.extrapolate_count(), count)
    // }
    // if !is_continuous(&map.cubes[0..count]) {
    //     panic!("not continuous {:?}", map)
    // }
    // for i in 1..count {
    //     if map.cubes[i - 1] >= map.cubes[i] {
    //         panic!("{} >= {} not sorted", map.cubes[i - 1], map.cubes[i])
    //     }
    // }
    let map = to_min_rot_points(map, dim, count);
    store.insert(map);
}

/// try expaning each cube into both x+1 and x-1, calculating new dimension
/// and ensuring x is never negative
#[inline]
fn expand_xs(dst: &mut HashSet<CubeMapPos>, seed: &CubeMapPos, shape: &Dim, count: usize) {
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
fn expand_ys(dst: &mut HashSet<CubeMapPos>, seed: &CubeMapPos, shape: &Dim, count: usize) {
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
fn expand_zs(dst: &mut HashSet<CubeMapPos>, seed: &CubeMapPos, shape: &Dim, count: usize) {
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
fn do_cube_expansion(dst: &mut HashSet<CubeMapPos>, seed: &CubeMapPos, shape: &Dim, count: usize) {
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
fn expand_cube_map(dst: &mut HashSet<CubeMapPos>, seed: &CubeMapPos, shape: &Dim, count: usize) {
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

fn enumerate_canonical_children(
    seed: &CubeMapPos,
    count: usize,
    target: usize,
    set_stack: &[RwLock<HashSet<CubeMapPos>>],
) -> usize {
    let mut children = set_stack[count].write();
    children.clear();
    let shape = seed.extrapolate_dim();
    expand_cube_map(&mut children, seed, &shape, count);
    children.retain(|child| is_canonical_root(child, count, seed));
    if count + 1 == target {
        children.len()
    } else {
        children
            .iter()
            .map(|child| enumerate_canonical_children(child, count + 1, target, set_stack))
            .sum()
    }
}

/// run pointlist based generation algorithm
pub fn gen_polycubes(
    n: usize,
    use_cache: bool,
    compression: Compression,
    parallel: bool,
    mut current: Vec<RawPCube>,
    calculate_from: usize,
) -> usize {
    let t1_start = Instant::now();

    let total: AtomicUsize = AtomicUsize::new(0);
    let work_count: AtomicUsize = AtomicUsize::new(0);

    let seed_count = current.len();
    let bar = make_bar(seed_count as u64);
    bar.set_message(format!(
        "seed subsets expanded for N = {}...",
        calculate_from - 1
    ));

    //convert input vector of NaivePolyCubes and convert them to
    if parallel {
        current.par_iter().for_each(|seed| {
            let seed = seed.into();
            let sets = (0..n)
                .map(|_| RwLock::new(HashSet::new()))
                .collect::<Vec<_>>();
            let children = enumerate_canonical_children(&seed, calculate_from - 1, n, &sets);
            total.fetch_add(children, Ordering::Relaxed);
            let steps = work_count.fetch_add(1, Ordering::Relaxed) + 1;
            let t1_now = Instant::now();
            let time = t1_now.duration_since(t1_start).as_secs() as usize;
            bar.set_message(format!(
                "seed subsets expanded for N = {}. est {}m remaining..",
                calculate_from - 1,
                ((time * seed_count) / steps - time) / 60
            ));
            bar.inc(1);
        });
    } else {
        current.iter().for_each(|seed| {
            let seed = seed.into();
            let sets = (0..n)
                .map(|_| RwLock::new(HashSet::new()))
                .collect::<Vec<_>>();
            let children = enumerate_canonical_children(&seed, calculate_from - 1, n, &sets);
            total.fetch_add(children, Ordering::Relaxed);
            let steps = work_count.fetch_add(1, Ordering::Relaxed) + 1;
            let t1_now = Instant::now();
            let time = t1_now.duration_since(t1_start).as_secs() as usize;
            bar.set_message(format!(
                "seed subsets expanded for N = {}. est {}m remaining..",
                calculate_from - 1,
                (time * seed_count) / steps / 60
            ));
            bar.inc(1);
        });
    }
    let count = total.load(Ordering::Relaxed);
    let t1_stop = Instant::now();
    let time = t1_stop.duration_since(t1_start).as_micros();
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
