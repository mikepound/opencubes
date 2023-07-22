use std::{cmp::max, time::Instant};

use crate::{
    make_bar,
    polycube_reps::{CubeMapPos, CubeMapPosPart, Dim},
    rotations::{rot_matrix_points, to_min_rot_points, MatrixCol},
    Compression,
};

use hashbrown::{HashMap, HashSet};
use indicatif::ProgressBar;
use opencubes::pcube::{PCubeFile, RawPCube};
use parking_lot::RwLock;
use rayon::prelude::*;

///structure to store the polycubes
/// stores the key and fist block index as a key
/// to the set of 15 block tails that correspond to that shape and start
/// used for reducing mutex preasure on insertion
/// used as buckets for parallelising
/// however both of these give suboptomal performance due to the uneven distribution
type MapStore = HashMap<(Dim, u16), RwLock<HashSet<CubeMapPosPart>>>;

/// helper function to not duplicate code for canonicalising polycubes
/// and storing them in the hashset
fn insert_map(store: &MapStore, dim: &Dim, map: &CubeMapPos, count: usize) {
    let map = to_min_rot_points(map, dim, count);
    let mut body = CubeMapPosPart { cubes: [0; 15] };
    for i in 1..16 {
        body.cubes[i - 1] = map.cubes[i];
    }
    match store.get(&(*dim, map.cubes[0])) {
        Some(map) => {
            map.write().insert(body);
        }
        None => {
            panic!(
                "shape {:?} data {} {:?} count {}",
                dim, map.cubes[0], body, count
            );
        }
    }
}

///linearly scan backwards to insertion point overwrites end of slice
#[inline]
fn array_insert(val: u16, arr: &mut [u16]) {
    for i in 1..(arr.len()) {
        if arr[arr.len() - 1 - i] > val {
            arr[arr.len() - i] = arr[arr.len() - 1 - i];
        } else {
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
fn expand_ys(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
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
fn expand_zs(dst: &MapStore, seed: &CubeMapPos, shape: &Dim, count: usize) {
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

/// helper for inner_exp in expand_cube_set it didnt like going directly in the closure
fn expand_cube_sub_set(
    (shape, start): &(Dim, u16),
    body: &RwLock<HashSet<CubeMapPosPart>>,
    count: usize,
    dst: &MapStore,
) {
    let mut seed = CubeMapPos {
        cubes: [*start, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    for seed_body in body.read().iter() {
        for i in 1..count {
            seed.cubes[i] = seed_body.cubes[i - 1];
        }
        expand_cube_map(dst, &seed, &shape, count);
    }
}

fn expand_cube_set(
    seeds: &MapStore,
    count: usize,
    dst: &mut MapStore,
    bar: &mut ProgressBar,
    parallel: bool,
) {
    // set up the dst sets before starting parallel processing so accessing doesnt block a global mutex
    for x in 0..=count + 1 {
        for y in 0..=(count + 1) / 2 {
            for z in 0..=(count + 1) / 3 {
                for i in 0..(y + 1) * 32 {
                    dst.insert((Dim { x, y, z }, i as u16), RwLock::new(HashSet::new()));
                }
            }
        }
    }

    let bar = RwLock::new(bar);
    bar.write()
        .set_message(format!("seed subsets expanded for N = {}...", count + 1));

    let inner_exp = |(ss, body)| {
        expand_cube_sub_set(ss, body, count, dst);
        bar.write().inc(1);
    };

    //use parallel iterator or not to run expand_cube_set
    if parallel {
        seeds.par_iter().for_each(inner_exp);
    } else {
        seeds.iter().for_each(inner_exp);
    }
    //retain only subsets that have polycubes
    dst.retain(|_, v| v.read().len() > 0);
}

/// count the number of polycubes across all subsets
fn count_polycubes(maps: &MapStore) -> usize {
    let mut total = 0;
    #[cfg(feature = "diagnostics")]
    for ((d, s), body) in maps.iter().rev() {
        println!(
            "({}, {}, {}) {} _> {}",
            d.x + 1,
            d.y + 1,
            d.z + 1,
            s,
            body.len()
        );
    }
    for (_, body) in maps.iter() {
        total += body.read().len()
    }
    total
}

/// distructively move the data from hashset to vector
fn move_polycubes_to_vec(maps: &mut MapStore) -> Vec<CubeMapPos> {
    let mut v = Vec::new();
    while let Some(((dim, head), body)) = maps.iter().next() {
        //extra scope to free lock and make borrow checker allow mutation of maps
        {
            let bod = body.read();
            let mut cmp = CubeMapPos { cubes: [0; 16] };
            cmp.cubes[0] = *head;
            for b in bod.iter() {
                for i in 0..15 {
                    cmp.cubes[i + 1] = b.cubes[i];
                }
                v.push(cmp);
            }
        }
        let dim = *dim;
        let head = *head;
        maps.remove(&(dim, head));
    }
    v
}

/// distructively move the data from hashset to vector
fn clone_polycubes_to_vec(maps: &mut MapStore) -> Vec<CubeMapPos> {
    let mut v = Vec::new();

    for ((_, head), body) in maps.iter() {
        //extra scope to free lock and make borrow checker allow mutation of maps
        {
            let bod = body.read();
            let mut cmp = CubeMapPos { cubes: [0; 16] };
            cmp.cubes[0] = *head;
            for b in bod.iter() {
                for i in 0..15 {
                    cmp.cubes[i + 1] = b.cubes[i];
                }
                v.push(cmp);
            }
        }
    }
    v
}

/// run pointlist based generation algorithm
pub fn gen_polycubes(
    n: usize,
    use_cache: bool,
    compression: Compression,
    parallel: bool,
    current: Vec<RawPCube>,
    calculate_from: usize,
) -> Vec<CubeMapPos> {
    let t1_start = Instant::now();

    //convert input vector of NaivePolyCubes and convert them to
    let mut seeds = MapStore::new();
    for seed in current.iter() {
        let seed: CubeMapPos = seed.into();
        let dim = seed.extrapolate_dim();
        if !seeds.contains_key(&(dim, seed.cubes[0])) {
            for i in 0..(dim.y * 32 + dim.x + 1) {
                seeds.insert((dim, i as u16), RwLock::new(HashSet::new()));
            }
        }
        insert_map(&seeds, &dim, &seed, calculate_from - 1);
    }
    drop(current);

    for i in calculate_from..=n as usize {
        let mut bar = make_bar(seeds.len() as u64);
        bar.set_message(format!("seed subsets expanded for N = {}...", i));
        let mut dst = MapStore::new();
        expand_cube_set(&mut seeds, i - 1, &mut dst, &mut bar, parallel);
        seeds = dst;

        if use_cache && i < n {
            let next = clone_polycubes_to_vec(&mut seeds);
            let name = &format!("cubes_{i}.pcube");
            if !std::fs::File::open(name).is_ok() {
                println!("Saving {} cubes to cache file", next.len());
                PCubeFile::write_file(
                    false,
                    compression.into(),
                    next.iter().map(|v| v.into()),
                    name,
                )
                .unwrap();
            } else {
                println!("Cache file already exists for N = {i}. Not overwriting.");
            }
        }

        let t1_stop = Instant::now();
        let time = t1_stop.duration_since(t1_start).as_micros();
        bar.set_message(format!(
            "Found {} unique expansions (N = {i}) in  {}.{:06}s",
            count_polycubes(&seeds),
            time / 1000000,
            time % 1000000
        ));

        bar.finish();
    }
    // exported eperately for memory concerns. already quite a lot more probably but not much I can do
    let next = move_polycubes_to_vec(&mut seeds);
    if use_cache {
        let name = &format!("cubes_{n}.pcube");
        if !std::fs::File::open(name).is_ok() {
            println!("Saving {} cubes to cache file", next.len());
            PCubeFile::write_file(
                false,
                compression.into(),
                next.iter().map(|v| v.into()),
                name,
            )
            .unwrap();
        } else {
            println!("Cache file already exists for N = {n}. Not overwriting.");
        }
    }
    next
    //count_polycubes(&seeds);
}
