use std::time::Instant;

use crate::polycubes::{
    pcube::RawPCube,
    point_list::{CubeMapPos, PointListMeta},
    Dim,
};

use hashbrown::{HashMap, HashSet};
use indicatif::ProgressBar;
use parking_lot::RwLock;
use rayon::prelude::*;

/// Structure to store sets of polycubes
pub struct MapStore {
    /// Stores the shape and fist block index as a key
    /// to the set of 15 block tails that correspond to that shape and start.
    /// used for reducing rwlock pressure on insertion
    /// used as buckets for parallelising
    /// however both of these give suboptomal performance due to the uneven distribution
    inner: HashMap<(Dim, u16), RwLock<HashSet<CubeMapPos<15>>>>,
}

impl MapStore {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    fn insert(&self, plm: &PointListMeta<16>) {
        let map = &plm.point_list;
        let dim = plm.dim;
        let count = plm.count;

        // Check if we don't already happen to be in the minimum rotation position.
        let mut body_maybemin = CubeMapPos::new();
        body_maybemin.cubes[0..count].copy_from_slice(&map.cubes[1..count + 1]);
        let dim_maybe = map.extrapolate_dim();

        // Weirdly enough, doing the copy and doing the lookup check this
        // way is faster than only copying if `inner` has en entry for
        // dim_maybe.
        if self
            .inner
            .get(&(dim_maybe, map.cubes[0]))
            .map(|v| v.read().contains(&body_maybemin))
            == Some(true)
        {
            return;
        }

        let map = map.to_min_rot_points(dim, count);

        let mut body = CubeMapPos::new();
        body.cubes[0..count].copy_from_slice(&map.cubes[1..count + 1]);

        let entry = self
            .inner
            .get(&(plm.dim, map.cubes[0]))
            .expect("Cube size does not have entry in destination map");

        entry.write().insert(body);
    }

    /// helper for inner_exp in expand_cube_set it didnt like going directly in the closure
    fn expand_cube_sub_set(
        &self,
        shape: Dim,
        first_cube: u16,
        body: impl Iterator<Item = CubeMapPos<15>>,
        count: usize,
    ) {
        let mut seed = CubeMapPos {
            cubes: [first_cube, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };

        for seed_body in body {
            for i in 1..count {
                seed.cubes[i] = seed_body.cubes[i - 1];
            }

            // body.cubes.copy_within(0..body.cubes.len() - 1, 1);
            let seed_meta = PointListMeta {
                point_list: seed,
                dim: shape,
                count,
            };
            seed_meta.expand().for_each(|plm| self.insert(&plm));
        }
    }

    fn expand_cube_set(self, count: usize, dst: &mut MapStore, bar: &ProgressBar, parallel: bool) {
        // set up the dst sets before starting parallel processing so accessing doesnt block a global mutex
        for x in 0..=count + 1 {
            for y in 0..=(count + 1) / 2 {
                for z in 0..=(count + 1) / 3 {
                    for i in 0..(y + 1) * 32 {
                        dst.inner
                            .insert((Dim { x, y, z }, i as u16), RwLock::new(HashSet::new()));
                    }
                }
            }
        }

        bar.set_message(format!("seed subsets expanded for N = {}...", count + 1));

        let inner_exp = |((shape, first_cube), body): (_, RwLock<HashSet<_>>)| {
            dst.expand_cube_sub_set(shape, first_cube, body.into_inner().into_iter(), count);
            bar.inc(1);
        };

        // Use parallel iterator or not to run expand_cube_set
        if parallel {
            self.inner.into_par_iter().for_each(inner_exp);
        } else {
            self.inner.into_iter().for_each(inner_exp);
        }

        //retain only subsets that have polycubes
        dst.inner.retain(|_, v| v.read().len() > 0);
    }

    /// Count the number of polycubes across all subsets
    fn count_polycubes(&self) -> usize {
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
        for (_, body) in self.inner.iter() {
            total += body.read().len()
        }

        total
    }

    /// Destructively move the data from hashset to vector
    pub fn into_vec(self) -> Vec<CubeMapPos<16>> {
        let mut v = Vec::with_capacity(self.count_polycubes());

        for ((_, head), body) in self.inner.into_iter() {
            let bod = body.read();
            let mut cmp = CubeMapPos::new();
            cmp.cubes[0] = head;
            for b in bod.iter() {
                for i in 0..15 {
                    cmp.cubes[i + 1] = b.cubes[i];
                }
                v.push(cmp);
            }
        }

        v
    }

    /// Copy the data from hashset to vector
    pub fn to_vec(&self) -> Vec<CubeMapPos<16>> {
        let mut v = Vec::with_capacity(self.count_polycubes());

        for ((_, head), body) in self.inner.iter() {
            let bod = body.read();
            let mut cmp = CubeMapPos::new();
            cmp.cubes[0] = *head;
            for b in bod.iter() {
                for i in 0..15 {
                    cmp.cubes[i + 1] = b.cubes[i];
                }
                v.push(cmp);
            }
        }

        v
    }
}

/// run pointlist based generation algorithm
pub fn gen_polycubes(
    n: usize,
    _use_cache: bool,
    parallel: bool,
    current: Vec<RawPCube>,
    calculate_from: usize,
    bar: &ProgressBar,
) -> Vec<CubeMapPos<16>> {
    let t1_start = Instant::now();

    //convert input vector of NaivePolyCubes and convert them to
    let mut seeds = MapStore::new();
    for seed in current.iter() {
        let seed: CubeMapPos<16> = seed.into();
        let dim = seed.extrapolate_dim();
        if !seeds.inner.contains_key(&(dim, seed.cubes[0])) {
            for i in 0..(dim.y * 32 + dim.x + 1) {
                seeds
                    .inner
                    .insert((dim, i as u16), RwLock::new(HashSet::new()));
            }
        }
        let seed_meta = PointListMeta {
            point_list: seed,
            dim,
            count: calculate_from - 1,
        };
        seeds.insert(&seed_meta);
    }
    drop(current);

    for i in calculate_from..=n as usize {
        bar.set_message(format!("seed subsets expanded for N = {}...", i));
        let mut dst = MapStore::new();
        seeds.expand_cube_set(i - 1, &mut dst, bar, parallel);
        seeds = dst;

        let t1_stop = Instant::now();
        let time = t1_stop.duration_since(t1_start).as_micros();
        bar.set_message(format!(
            "Found {} unique expansions (N = {i}) in {}.{:06}s",
            seeds.count_polycubes(),
            time / 1000000,
            time % 1000000
        ));

        bar.finish();
    }

    seeds.into_vec()
}
