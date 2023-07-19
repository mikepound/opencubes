use std::time::Instant;

use opencubes::naive_polycube::NaivePolyCube;

use crate::{Compression, polycube_reps::naive_to_map_pos, make_bar};





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
    current.drop();

    
    next
    //count_polycubes(&seeds);
}