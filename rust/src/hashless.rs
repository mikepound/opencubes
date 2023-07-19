use std::time::Instant;

use opencubes::naive_polycube::NaivePolyCube;

use crate::{Compression, polycube_reps::{naive_to_map_pos, CubeMapPos}, make_bar, rotations::to_min_rot_points};

fn is_continuous(polycube: &CubeMapPos) -> bool {
    todo!()
}

fn canonical_root(exp: &CubeMapPos, count: usize) -> CubeMapPos {
    let root = CubeMapPos {cubes: [0;16]};
    for sub_cube_id in 0..count {
        let mut root_candidate = CubeMapPos {cubes: [0;16]};
        let mut candidate_ptr = 0;
        for i in 0..count {
            if i != sub_cube_id {
                root_candidate.cubes[candidate_ptr] = exp.cubes[i];
                candidate_ptr += 1;
            }
        }
        if is_continuous(&root_candidate) {
            continue;
        }
        let mrp = to_min_rot_points(&root_candidate, root_candidate.shape(), count - 1);
    }
    root
}



/// run pointlist based generation algorithm
pub fn gen_polycubes(
    n: usize,
    use_cache: bool,
    compression: Compression,
    parallel: bool,
    mut current: Vec<NaivePolyCube>,
    calculate_from: usize,
) -> usize {
    let t1_start = Instant::now();

    //convert input vector of NaivePolyCubes and convert them to
    let mut seeds = Vec::new();
    for seed in current.iter() {
        let (seed, dim) = naive_to_map_pos(seed);
        seeds.push((seed, dim));
    }
    drop(current);
    let count = 0;

    
    count
    //count_polycubes(&seeds);
}