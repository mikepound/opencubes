#[cfg(feature = "diagnostics")]
#[cfg(not(feature = "smallset"))]
use std::collections::HashMap;
use std::{cmp::max, collections::HashSet, time::Instant};

use indicatif::ProgressBar;

use crate::{
    make_bar,
    polycube_reps::CubeMap,
    rotations::{self, rot_matrix, MatrixCol},
};

#[cfg(feature = "diagnostics")]
#[cfg(feature = "size16")]
pub static MAX_X: usize = 16;

#[cfg(feature = "diagnostics")]
#[cfg(not(feature = "size16"))]
pub static MAX_X: usize = 32;

#[cfg(feature = "smallset")]
///converts a cube map to  map pos for hashset storage slow (+10% runtime combined with decode last measured)
fn cube_map_to_cube_map_pos(map: &CubeMap) -> CubeMapPos {
    let mut pos = CubeMapPos { cubes: [0; 16] };
    let mut i = 0;
    for z in 0..=map.z as usize {
        for y in 0..=map.y as usize {
            for x in 0..=map.x as usize {
                if map.get_block(x, y, z) == 1 {
                    pos.cubes[i] = ((z << 10) | (y << 5) | x) as u16;
                    i += 1;
                }
            }
        }
    }
    pos.cubes[i - 1] |= 0x8000;
    #[cfg(feature = "diagnostics")]
    {
        let a = cube_map_from_cube_map_pos(&pos);
        if a != *map {
            panic!("{:?} {:?} unequal", a, map);
        }
    }
    pos
}

#[cfg(feature = "smallset")]
///converts a mappos from hashset storage to a cube map
fn cube_map_from_cube_map_pos(map: &CubeMapPos) -> CubeMap {
    let mut dst = CubeMap {
        x: 0,
        y: 0,
        z: 0,
        cube_map: [0; 36],
    };
    let mut i = 0;
    //use compound condition as do while
    //iterate once for the bounds and once for the data
    while {
        let v = map.cubes[i];
        let x = (v & 0x1f) as usize;
        let y = ((v >> 5) & 0x1f) as usize;
        let z = ((v >> 10) & 0x1f) as usize;
        dst.x = max(dst.x, x as u32);
        dst.y = max(dst.y, y as u32);
        dst.z = max(dst.z, z as u32);
        i += 1;
        map.cubes[i - 1] >> 15 != 1
    } {}
    i = 0;
    //do {} while (map.cubes[i - 1] >> 15 != 1);
    while {
        let v = map.cubes[i];
        let x = (v & 0x1f) as usize;
        let y = ((v >> 5) & 0x1f) as usize;
        let z = ((v >> 10) & 0x1f) as usize;
        dst.set_block(x, y, z);
        i += 1;
        map.cubes[i - 1] >> 15 != 1
    } {}
    dst
}

#[cfg(feature = "smallset")]
type CubeEncoding = CubeMapPos;
#[cfg(not(feature = "smallset"))]
type CubeEncoding = CubeMap;

#[inline]
fn insert_map(
    map: &CubeMap,
    seen: &mut HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
) {
    let work_map = rotations::to_min_rot(map);
    #[cfg(feature = "diagnostics")]
    {
        if map.count_cubes() != depth {
            panic!("{:?} doesnt have {} cubes", map, depth)
        }
        if !map.validate_bounds() {
            panic!("{:?} has blocks out of bounds", map)
        }
        if !map.validate_continuity() {
            panic!("{:?} non continuous", map)
        }
    }
    #[cfg(feature = "smallset")]
    let work_map = cube_map_to_cube_map_pos(&work_map);
    seen.insert(work_map);
}

/// insert a cube towards +X
#[inline]
fn expand_cube_map_left(map: &CubeMap, yz: usize, offset: u32) -> CubeMap {
    let mut work_map = *map;
    work_map.cube_map[yz] |= 1 << offset;
    work_map.x = max(work_map.x, offset);
    work_map
}

/// insert a cube towards -X
#[inline]
fn expand_cube_map_right(map: &CubeMap, yz: usize, offset: u32) -> CubeMap {
    let mut work_map = *map;
    if offset == 0 {
        for i in 0..(((map.y + 1) * (map.z + 1)) as usize) {
            work_map.cube_map[i] = work_map.cube_map[i] << 1;
        }
        work_map.cube_map[yz] |= 1;
        work_map.x += 1;
    } else {
        work_map.cube_map[yz] |= 1 << (offset - 1);
    }
    work_map
}

/// insert a cube towards +Y
#[inline]
fn expand_cube_map_up(map: &CubeMap, y: usize, z: usize, offset: u32) -> CubeMap {
    let mut work_map = *map;
    if y > (work_map.y as usize) {
        for y in 0..((map.y + 1) as usize) {
            for z in 0..((map.z + 1) as usize) {
                work_map.cube_map[z * (map.y as usize + 2) + y] =
                    map.cube_map[z * (map.y as usize + 1) + y];
            }
        }
        work_map.y += 1;
        for i in 0..(work_map.z as usize + 1) {
            work_map.cube_map[(i + 1) * (work_map.y as usize + 1) - 1] = 0;
        }
    }
    work_map.cube_map[z * (work_map.y as usize + 1) + y] |= 1 << offset;
    work_map
}

/// insert a cube towards -Y
#[inline]
fn expand_cube_map_down(map: &CubeMap, y: usize, z: usize, offset: u32) -> CubeMap {
    let mut work_map = *map;
    if y == 0 {
        for y in 0..((map.y + 1) as usize) {
            for z in 0..((map.z + 1) as usize) {
                work_map.cube_map[z * (map.y as usize + 2) + y + 1] =
                    map.cube_map[z * (map.y as usize + 1) + y];
            }
        }
        work_map.y += 1;
        for i in 0..(work_map.z as usize + 1) {
            work_map.cube_map[i * (work_map.y as usize + 1)] = 0;
        }
        work_map.cube_map[z * (work_map.y as usize + 1)] |= 1 << offset;
    } else {
        work_map.cube_map[z * (map.y as usize + 1) + y - 1] |= 1 << offset;
    }
    work_map
}

/// insert a cube towards +Z
#[inline]
fn expand_cube_map_in(map: &CubeMap, y: usize, z: usize, offset: u32) -> CubeMap {
    let mut work_map = *map;
    work_map.cube_map[z * (map.y as usize + 1) + y] |= 1 << offset;
    work_map.z = max(work_map.z, z as u32);
    work_map
}

/// insert a cube towards -Z
#[inline]
fn expand_cube_map_out(map: &CubeMap, y: usize, z: usize, offset: u32) -> CubeMap {
    let mut work_map = *map;
    if z == 0 {
        for i in 0..(((map.y + 1) * (map.z + 1)) as usize) {
            work_map.cube_map[i + map.y as usize + 1] = map.cube_map[i];
        }
        work_map.z += 1;
        for i in 0..(work_map.y as usize + 1) {
            work_map.cube_map[i] = 0;
        }
        work_map.cube_map[y] |= 1 << offset;
    } else {
        work_map.cube_map[(z - 1) * (map.y as usize + 1) + y] |= 1 << offset;
    }
    work_map
}

/// expand each cube +/-1 X where possible
#[inline]
fn expand_xs(
    map: &CubeMap,
    seen: &mut HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
) {
    for yz in 0..(((map.y + 1) * (map.z + 1)) as usize) {
        let left_bits = ((map.cube_map[yz] << 1) | map.cube_map[yz]) ^ map.cube_map[yz];
        let right_bits = ((map.cube_map[yz] << 1) | map.cube_map[yz]) ^ (map.cube_map[yz] << 1);
        for xoff in 1..(map.x + 2) {
            //start at 1 because shifting left cant be in the zero bit
            if left_bits & (1 << xoff) != 0 {
                insert_map(
                    &expand_cube_map_left(map, yz, xoff),
                    seen,
                    #[cfg(feature = "diagnostics")]
                    depth,
                );
            }
        }
        for xoff in 0..(map.x + 1) {
            if right_bits & (1 << xoff) != 0 {
                insert_map(
                    &expand_cube_map_right(map, yz, xoff),
                    seen,
                    #[cfg(feature = "diagnostics")]
                    depth,
                );
            }
        }
    }
}

/// expand each cube +/-1 Y where possible
#[inline]
fn expand_ys(
    map: &CubeMap,
    seen: &mut HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
) {
    for z in 0..=map.z as usize {
        for y in 0..=map.y as usize {
            let up_bits = if y == map.y as usize {
                map.cube_map[z * (map.y as usize + 1) + y]
            } else {
                map.cube_map[z * (map.y as usize + 1) + y]
                    & (!map.cube_map[z * (map.y as usize + 1) + y + 1])
            };
            let down_bits = if y == 0 {
                map.cube_map[z * (map.y as usize + 1)]
            } else {
                map.cube_map[z * (map.y as usize + 1) + y]
                    & (!map.cube_map[z * (map.y as usize + 1) + y - 1])
            };
            for xoff in 0..=map.x {
                //start at 1 because shifting left cant be in the zero bit
                if up_bits & (1 << xoff) != 0 {
                    insert_map(
                        &expand_cube_map_up(map, y + 1, z, xoff),
                        seen,
                        #[cfg(feature = "diagnostics")]
                        depth,
                    );
                }
                if down_bits & (1 << xoff) != 0 {
                    insert_map(
                        &expand_cube_map_down(map, y, z, xoff),
                        seen,
                        #[cfg(feature = "diagnostics")]
                        depth,
                    );
                }
            }
        }
    }
}

/// expand each cube +/-1 Z where possible
#[inline]
fn expand_zs(
    map: &CubeMap,
    seen: &mut HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
) {
    for z in 0..=map.z as usize {
        for y in 0..=map.y as usize {
            let in_bits = map.cube_map[z * (map.y as usize + 1) + y]
                & (!map.cube_map[(z + 1) * (map.y as usize + 1) + y]);
            let out_bits = if z == 0 {
                map.cube_map[y]
            } else {
                map.cube_map[z * (map.y as usize + 1) + y]
                    & (!map.cube_map[(z - 1) * (map.y as usize + 1) + y])
            };
            for xoff in 0..(map.x + 1) {
                if in_bits & (1 << xoff) != 0 {
                    insert_map(
                        &expand_cube_map_in(map, y, z + 1, xoff),
                        seen,
                        #[cfg(feature = "diagnostics")]
                        depth,
                    );
                }
                if out_bits & (1 << xoff) != 0 {
                    insert_map(
                        &expand_cube_map_out(map, y, z, xoff),
                        seen,
                        #[cfg(feature = "diagnostics")]
                        depth,
                    );
                }
            }
        }
    }
}

/// expand in X, Y and Z abiding by the X >= Y >= Z constraint
#[inline]
fn do_cube_expansion(
    map: &CubeMap,
    seen: &mut HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
) {
    expand_xs(
        map,
        seen,
        #[cfg(feature = "diagnostics")]
        depth,
    );
    if map.y < map.x {
        expand_ys(
            map,
            seen,
            #[cfg(feature = "diagnostics")]
            depth,
        );
    }
    if map.z < map.y {
        expand_zs(
            map,
            seen,
            #[cfg(feature = "diagnostics")]
            depth,
        );
    }
}

/// expand cube, rotate around square faces to catch adgecases that were getting missed due to the X >= Y >= Z constraint
#[inline]
fn expand_cube_map(
    map: &CubeMap,
    seen: &mut HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
) {
    do_cube_expansion(
        map,
        seen,
        #[cfg(feature = "diagnostics")]
        depth,
    );
    #[cfg(feature = "diagnostics")]
    if map.count_cubes() != depth - 1 {
        panic!("{:?} doesnt have {} cubes", map, depth - 1)
    }
    if map.x == map.y && map.x > 0 {
        let mut rot = CubeMap {
            x: map.x,
            y: map.y,
            z: map.z,
            cube_map: [0; 36],
        };
        rot_matrix(map, &mut rot, MatrixCol::YN, MatrixCol::XN, MatrixCol::ZN);
        #[cfg(feature = "diagnostics")]
        if rot.count_cubes() != depth - 1 {
            panic!("{:?} doesnt have {} cubes", rot, depth - 1)
        }
        do_cube_expansion(
            &rot,
            seen,
            #[cfg(feature = "diagnostics")]
            depth,
        );
    }
    if map.y == map.z && map.y > 0 {
        let mut rot = CubeMap {
            x: map.x,
            y: map.y,
            z: map.z,
            cube_map: [0; 36],
        };
        rot_matrix(map, &mut rot, MatrixCol::XN, MatrixCol::ZP, MatrixCol::YP);
        #[cfg(feature = "diagnostics")]
        if rot.count_cubes() != depth - 1 {
            panic!("{:?} doesnt have {} cubes", rot, depth - 1)
        }
        do_cube_expansion(
            &rot,
            seen,
            #[cfg(feature = "diagnostics")]
            depth,
        );
    }
    if map.x == map.z && map.x > 0 {
        let mut rot = CubeMap {
            x: map.x,
            y: map.y,
            z: map.z,
            cube_map: [0; 36],
        };
        rot_matrix(map, &mut rot, MatrixCol::ZP, MatrixCol::YP, MatrixCol::XN);
        #[cfg(feature = "diagnostics")]
        if rot.count_cubes() != depth - 1 {
            panic!("{:?} doesnt have {} cubes", rot, depth - 1)
        }
        do_cube_expansion(
            &rot,
            seen,
            #[cfg(feature = "diagnostics")]
            depth,
        );
    }
}

#[cfg(feature = "diagnostics")]
#[cfg(not(feature = "smallset"))]
fn to_dim(cm: &CubeMap) -> Dim {
    Dim {
        x: cm.x as usize + 1,
        y: cm.y as usize + 1,
        z: cm.z as usize + 1,
    }
}

/// expand all polycubes in set n-1 to get set n
fn expand_cube_set(
    in_set: &HashSet<CubeEncoding>,
    #[cfg(feature = "diagnostics")] depth: usize,
    out_set: &mut HashSet<CubeEncoding>,
    bar: &mut ProgressBar,
) {
    let mut i = 0;
    for map in in_set.iter() {
        #[cfg(feature = "smallset")]
        let map = &cube_map_from_cube_map_pos(map);
        expand_cube_map(
            map,
            out_set,
            #[cfg(feature = "diagnostics")]
            depth,
        );
        i += 1;
        if i == 100 {
            bar.inc(100);
            i = 0;
        }
    }
    bar.inc(i);
    #[cfg(feature = "diagnostics")]
    #[cfg(not(feature = "smallset"))]
    {
        let mut shape_map = HashMap::new();
        for map in out_set.iter() {
            if map.count_cubes() != depth {
                panic!("{:?} doesnt have {} cubes", map, depth)
            }
            let dim = to_dim(map);
            shape_map.insert(
                dim,
                match shape_map.get(&dim) {
                    Some(res) => res + 1,
                    None => 1,
                },
            );
        }
        for (s, map) in shape_map.iter() {
            println!("{}, {}, {} -> {:?}", s.x, s.y, s.z, map);
        }
    }
}

pub fn gen_polycubes(n: usize) -> usize {
    let unit_cube = CubeMap {
        x: 1,
        y: 0,
        z: 0,
        cube_map: [
            3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ],
    };
    let t1_start = Instant::now();
    let mut seeds = HashSet::new();
    let mut dst = HashSet::new();
    insert_map(
        &unit_cube,
        &mut seeds,
        #[cfg(feature = "diagnostics")]
        2,
    );
    for i in 3..=n as usize {
        let mut bar = make_bar(seeds.len() as u64);
        bar.set_message(format!("seed subsets expanded for N = {}...", i));
        expand_cube_set(
            &seeds,
            #[cfg(feature = "diagnostics")]
            i,
            &mut dst,
            &mut bar,
        );
        //if diagnostics enabled panic if the returned values are wrong
        #[cfg(feature = "diagnostics")]
        if i == 3 && dst.len() != 2 {
            panic!("{} supposed to have {} elems not {}", i, 2, dst.len())
        } else if i == 4 && dst.len() != 8 {
            panic!("{} supposed to have {} elems not {}", i, 8, dst.len())
        } else if i == 5 && dst.len() != 29 {
            panic!("{} supposed to have {} elems not {}", i, 29, dst.len())
        } else if i == 6 && dst.len() != 166 {
            panic!("{} supposed to have {} elems not {}", i, 166, dst.len())
        } else if i == 7 && dst.len() != 1023 {
            panic!("{} supposed to have {} elems not {}", i, 1023, dst.len())
        } else if i == 8 && dst.len() != 6922 {
            panic!("{} supposed to have {} elems not {}", i, 6922, dst.len())
        }
        let tmp = seeds;
        seeds = dst;
        dst = tmp;
        dst.clear();
        dst.reserve(seeds.len() * 8);
        let t1_stop = Instant::now();
        let time = t1_stop.duration_since(t1_start).as_micros();
        bar.set_message(format!(
            "Found {} unique expansions (N = {i}) in  {}.{:06}s",
            seeds.len(),
            time / 1000000,
            time % 1000000
        ));

        bar.finish();
    }
    seeds.len()
}
