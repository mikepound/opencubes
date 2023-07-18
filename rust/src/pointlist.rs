use std::{time::Instant, cmp::max, sync::Mutex};

use crate::{rotations::{MatrixCol, rot_matrix_points, to_min_rot_points}, polycube_reps::{Dim, CubeMapPos, CubeMapPosPart}};

use hashbrown::{HashSet, HashMap};
use rayon::prelude::*;

///structure to store the polycubes
/// stores the key and fist block index as a key
/// to the set of 15 block tails that correspond to that shape and start
/// used for reducing mutex preasure on insertion
/// used as buckets for parallelising
/// however both of these give suboptomal performance due to the uneven distribution
type MapStore =  HashMap<(Dim, u16), Mutex<HashSet<CubeMapPosPart>>>;

/// helper function to not duplicate code for canonicalising polycubes
/// and storing them in the hashset
fn insert_map(store: &MapStore, dim: &Dim, map: &CubeMapPos, count: usize) {
    let map = to_min_rot_points(map, dim, count);
    let mut body = CubeMapPosPart {cubes: [0; 15]};
    for i in 1..16 {
        body.cubes[i - 1] = map.cubes[i];
    }
    match store.get(&(*dim, map.cubes[0])) {
        Some(map) => {
            map.lock().unwrap().insert(body);
        }
        None => {
            panic!("shape {:?} data {} {:?} count {}", dim, map.cubes[0], body, count);
        },
    }
}

///linearly scan backwards to insertion point overwrites end of slice
#[inline]
fn array_insert(val: u16, arr: &mut [u16]) {
    for i in 1..(arr.len()) {
        if arr[arr.len() - 1 - i] > val {
            arr[arr.len() - i] = arr[arr.len() - 1 - i];
        }
        else {
            arr[arr.len() - i] = val;
            return;
        }
    }
    arr[0] = val;
}

/// moves contents of slice to index x+1, x==0 remains
#[inline]
fn array_shift(arr: &mut [u16]) {
    for i in 1..(arr.len()) {
        arr[arr.len() - i] = arr[arr.len() - 1 - i];
    }
}

/// try expaning each cube into both x+1 and x-1, calculating new dimension
/// and ensuring x is never negative
#[inline]
fn expand_xs(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
    for (i, coord) in seed.cubes[0..count].iter().enumerate() {
        if !seed.cubes[(i+1)..count].contains(&(coord + 1)) {
            let mut new_shape = *shape;
            let mut exp_map = *seed;

            array_insert(coord + 1, &mut exp_map.cubes[i..=count]);
            new_shape.x = max(new_shape.x, ((coord+1) & 0x1f) as usize);
            insert_map(dst, &new_shape, &exp_map, count+1)
        }
        if coord & 0x1f != 0 {
            if !seed.cubes[0..i].contains(&(coord - 1)) {
                let mut exp_map = *seed;
                //faster move of top half hopefully
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(coord - 1, &mut exp_map.cubes[0..=i]);
                insert_map(dst, shape, &exp_map, count+1)
            }
        }
        else {
            let mut new_shape = *shape;
            new_shape.x  += 1;
            let mut exp_map = *seed;
            for i in 0..count {
                exp_map.cubes[i] += 1;
            }
            array_shift(&mut exp_map.cubes[i..=count]);
            array_insert(*coord, &mut exp_map.cubes[0..=i]);
            insert_map(dst, &new_shape, &exp_map, count+1)
        }

    }
}

/// try expaning each cube into both y+1 and y-1, calculating new dimension
/// and ensuring y is never negative
#[inline]
fn expand_ys(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
    for (i, coord) in seed.cubes[0..count].iter().enumerate() {
        if !seed.cubes[(i+1)..count].contains(&(coord + (1 << 5))) {
            let mut new_shape = *shape;
            let mut exp_map = *seed;
            array_insert(coord + (1 << 5), &mut exp_map.cubes[i..=count]);
            new_shape.y = max(new_shape.y, (((coord >> 5)+1) & 0x1f) as usize);
            insert_map(dst, &new_shape, &exp_map, count+1)
        }
        if (coord >> 5) & 0x1f != 0 {
            if !seed.cubes[0..i].contains(&(coord - (1 << 5))) {
                let mut exp_map = *seed;
                //faster move of top half hopefully
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(coord - (1 << 5), &mut exp_map.cubes[0..=i]);
                insert_map(dst, shape, &exp_map, count+1)
            }
        }
        else {
            let mut new_shape = *shape;
            new_shape.y  += 1;
            let mut exp_map = *seed;
            for i in 0..count {
                exp_map.cubes[i] += 1 << 5;
            }
            array_shift(&mut exp_map.cubes[i..=count]);
            array_insert(*coord, &mut exp_map.cubes[0..=i]);
            insert_map(dst, &new_shape, &exp_map, count+1)
        }
    }
}

/// try expaning each cube into both z+1 and z-1, calculating new dimension
/// and ensuring z is never negative
#[inline]
fn expand_zs(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
    for (i, coord) in seed.cubes[0..count].iter().enumerate() {
        if !seed.cubes[(i+1)..count].contains(&(coord + (1 << 10))) {
            let mut new_shape = *shape;
            let mut exp_map = *seed;
            array_insert(coord + (1 << 10), &mut exp_map.cubes[i..=count]);
            new_shape.z = max(new_shape.z, (((coord >> 10)+1) & 0x1f) as usize);
            insert_map(dst, &new_shape, &exp_map, count+1)
        }
        if (coord >> 10) & 0x1f != 0 {
            if !seed.cubes[0..i].contains(&(coord - (1 << 10))) {
                let mut exp_map = *seed;
                //faster move of top half hopefully
                array_shift(&mut exp_map.cubes[i..=count]);
                array_insert(coord - (1 << 10), &mut exp_map.cubes[0..=i]);
                insert_map(dst, shape, &exp_map, count+1)
            }
        }
        else {
            let mut new_shape = *shape;
            new_shape.z  += 1;
            let mut exp_map = *seed;
            for i in 0..count {
                exp_map.cubes[i] += 1 << 10;
            }
            array_shift(&mut exp_map.cubes[i..=count]);
            array_insert(*coord, &mut exp_map.cubes[0..=i]);
            insert_map(dst, &new_shape, &exp_map, count+1)
        }
    }
}

/// reduce number of expansions needing to be performed based on
/// X >= Y >= Z constraint on Dim
#[inline]
fn do_cube_expansion(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
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
fn expand_cube_map(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
    if shape.x == shape.y && shape.x > 0 {
        let rotz = rot_matrix_points(seed, shape, count,
            MatrixCol::YN, MatrixCol::XN, MatrixCol::ZN, 1025);
        do_cube_expansion(dst, &rotz, shape, count);
    }
    if shape.y == shape.z && shape.y > 0 {
        let rotx = rot_matrix_points(seed, shape, count,
            MatrixCol::XN, MatrixCol::ZP, MatrixCol::YP, 1025);
        do_cube_expansion(dst, &rotx, shape, count);
    }
    if shape.x == shape.z && shape.x > 0 {
        let roty = rot_matrix_points(seed, shape, count,
            MatrixCol::ZP, MatrixCol::YP, MatrixCol::XN, 1025);
        do_cube_expansion(dst, &roty, shape, count);
    }
    do_cube_expansion(dst, seed, shape, count);
}

/// helper for inner_exp in expand_cube_set it didnt like going directly in the closure
fn expand_cube_sub_set((shape, start) : &(Dim, u16), body: &Mutex<HashSet<CubeMapPosPart>>, count: usize, dst: &MapStore) {
    let mut seed = CubeMapPos {
        cubes: [*start, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    };
    for seed_body in body.lock().unwrap().iter() {
        for i in 1..count {
            seed.cubes[i] = seed_body.cubes[i - 1];
        }
        expand_cube_map(dst, &seed, &shape, count);
    }
}

fn expand_cube_set(seeds: &MapStore, count: usize, dst: &mut MapStore, parallel: bool) {
    // set up the dst sets before starting parallel processing so accessing doesnt block a global mutex
    for x in 0..=count+1 {
        for y in 0..=(count+1)/2 {
            for z in 0..=(count+1)/3 {
                for i in 0..(y+1) * 32 {
                    dst.insert((Dim{x, y, z,}, i as u16), Mutex::new(HashSet::new()));
                }
            }
        }
    }

    let inner_exp = |(ss, body)|{
        expand_cube_sub_set(ss, body, count, dst);
    };

    //use parallel iterator or not to run expand_cube_set
    if parallel {
        seeds.par_iter().for_each(inner_exp);
    }
    else {
        seeds.iter().for_each(inner_exp);
    }
    //retain only subsets that have polycubes
    dst.retain(|_, v|{v.lock().unwrap().len() > 0});
}

/// count the number of polycubes across all subsets
fn count_polycubes(maps: &MapStore) -> usize {
    let mut total = 0;
    #[cfg(feature = "diagnostics")]
    for ((d,s), body) in maps.iter().rev() {
        println!("({}, {}, {}) {} _> {}", d.x+1, d.y+1, d.z+1, s, body.len());
    }
    for (_, body) in maps.iter() {
        total += body.lock().unwrap().len()
    }
    total
}

/// run pointlist based generation algorithm
pub fn gen_polycubes(n: usize, parallel: bool) -> usize {
    let unit_cube = CubeMapPos {
        cubes: [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    let t1_start = Instant::now();
    let mut seeds = MapStore::new();
    seeds.insert((Dim{x: 1, y: 0, z: 0}, 0), Mutex::new(HashSet::new()));
    insert_map(&seeds, &Dim {x: 1, y: 0, z: 0}, &unit_cube, 2);
    for i in 3..=n as usize {
        let mut dst = MapStore::new();
        expand_cube_set(&mut seeds, i-1, &mut dst, parallel);
        seeds = dst;
        let t1_stop = Instant::now();
        let time = t1_stop.duration_since(t1_start).as_micros();
        println!("Found {} unique polycube(s) at n = {} seeds_len {}", count_polycubes(&seeds), i, seeds.len());
        println!("Elapsed time: {}.{:06}s", time / 1000000, time % 1000000);
    }
    count_polycubes(&seeds)
}
