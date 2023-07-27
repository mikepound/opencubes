use std::{io::ErrorKind, sync::Arc, time::Instant};

use opencubes::{
    hashless::MapStore,
    iterator::{indicatif::PolycubeProgressBarIter, *},
    naive_polycube::NaivePolyCube,
    pcube::{PCubeFile, RawPCube},
    pointlist,
    polycube_reps::CubeMapPos,
    rotation_reduced,
};

use crate::{finish_bar, make_bar, unknown_bar, Compression, EnumerateOpts, EnumerationMode};

use rayon::{iter::ParallelBridge, prelude::ParallelIterator};

#[derive(Clone)]
struct AllUniques {
    current: Arc<std::vec::Vec<RawPCube>>,
    offset: usize,
    n: usize,
}

impl Iterator for AllUniques {
    type Item = RawPCube;

    fn next(&mut self) -> Option<Self::Item> {
        let output = self.current.get(self.offset)?.clone();
        self.offset += 1;
        Some(output)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.current.len() - self.offset;
        (len, Some(len))
    }
}

impl ExactSizeIterator for AllUniques {}

impl PolycubeIterator for AllUniques {
    fn is_canonical(&self) -> bool {
        false
    }

    fn n_hint(&self) -> Option<usize> {
        Some(self.n)
    }
}

impl AllPolycubeIterator for AllUniques {}
impl UniquePolycubeIterator for AllUniques {}
impl AllUniquePolycubeIterator for AllUniques {}

fn save_to_cache(
    compression: Compression,
    n: usize,
    // Ideally, this would be `AllUniquePolycubeIterator` but it's
    // a bit unwieldy
    cubes: impl Iterator<Item = RawPCube> + ExactSizeIterator,
) {
    let name = &format!("cubes_{n}.pcube");
    if !std::fs::File::open(name).is_ok() {
        println!("Saving {} cubes to cache file", cubes.len());
        PCubeFile::write_file(false, compression.into(), cubes, name).unwrap();
    } else {
        println!("Cache file already exists for N = {n}. Not overwriting.");
    }
}

fn load_cache_file(n: usize) -> Option<PCubeFile> {
    let name = format!("cubes_{n}.pcube");

    match PCubeFile::new_file(&name) {
        Ok(file) => Some(file),
        Err(e) => {
            if e.kind() == ErrorKind::InvalidData || e.kind() == ErrorKind::Other {
                println!("Enountered invalid cache file {name}. Error: {e}.");
            }
            None
        }
    }
}

enum CacheOrbase {
    Cache(opencubes::pcube::AllUnique),
    Base(bool),
}

impl Iterator for CacheOrbase {
    type Item = RawPCube;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CacheOrbase::Cache(cache) => cache.next(),
            CacheOrbase::Base(v) if v == &false => {
                *v = true;
                let mut base = RawPCube::new_empty(1, 1, 1);
                base.set(0, 0, 0, true);
                Some(base)
            }
            CacheOrbase::Base(_) => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            CacheOrbase::Cache(c) => c.size_hint(),
            CacheOrbase::Base(_) => (1, Some(1)),
        }
    }
}

impl PolycubeIterator for CacheOrbase {
    fn is_canonical(&self) -> bool {
        match self {
            CacheOrbase::Cache(c) => c.is_canonical(),
            CacheOrbase::Base(_) => true,
        }
    }

    fn n_hint(&self) -> Option<usize> {
        match self {
            CacheOrbase::Cache(c) => Some(c.n()),
            CacheOrbase::Base(_) => Some(1),
        }
    }
}

impl UniquePolycubeIterator for CacheOrbase {}
impl AllPolycubeIterator for CacheOrbase {}
impl AllUniquePolycubeIterator for CacheOrbase {}

/// load largest findable cachefile with size <= n - 1 into a vec
/// returns a vec and the next order above the found cache file
fn load_cache(n: usize) -> CacheOrbase {
    let calculate_from = 2;

    for n in (calculate_from..n).rev() {
        let cache = if let Some(file) = load_cache_file(n) {
            file
        } else {
            continue;
        };

        println!("Found cache for N = {n}.");
        return CacheOrbase::Cache(cache.assume_all_unique());
    }

    println!(
        "No cache file found for size <= {}. Starting from N = 1",
        n.saturating_sub(1)
    );

    CacheOrbase::Base(false)
}

fn unique_expansions(
    save_cache: bool,
    n: usize,
    compression: Compression,
    current: impl AllUniquePolycubeIterator,
    parallel: bool,
) -> Vec<RawPCube> {
    if n == 0 {
        return Vec::new();
    }

    let calculate_from = current.n();
    let current = current.collect();

    let mut current = AllUniques {
        current: Arc::new(current),
        offset: 0,
        n: calculate_from,
    };

    let mut i = calculate_from;

    loop {
        let bar = make_bar(current.len() as u64);
        bar.set_message(format!("Expanding base polycubes of N = {i}..."));

        let start = Instant::now();

        let with_bar = PolycubeProgressBarIter::new(bar.clone(), current);
        let next: Vec<RawPCube> = if parallel {
            NaivePolyCube::unique_expansions_rayon(with_bar).collect()
        } else {
            NaivePolyCube::unique_expansions(with_bar).collect()
        };

        finish_bar(&bar, start.elapsed(), next.len(), i + 1);

        if save_cache {
            save_to_cache(compression, i + 1, next.iter().map(Clone::clone));
        }

        i += 1;

        if n.saturating_sub(i) == 0 {
            return next;
        } else {
            current = AllUniques {
                current: Arc::new(next),
                offset: 0,
                n: i + 1,
            };
        }
    }
}

/// run pointlist based generation algorithm
pub fn enumerate_hashless(
    n: usize,
    parallel: bool,
    current: impl AllUniquePolycubeIterator + Send,
) -> usize {
    let t1_start = Instant::now();

    let start_n = current.n();
    let bar = if let (_, Some(max)) = current.size_hint() {
        make_bar(max as u64)
    } else {
        unknown_bar()
    };

    bar.set_message(format!("Expanding seeds of N = {}...", start_n));

    let process = |seed: RawPCube| {
        let seed: CubeMapPos<32> = seed.into();
        let children = MapStore::enumerate_canonical_children_min_mem(&seed, start_n, n);
        bar.inc(1);
        children
    };

    let count: usize = if parallel {
        current.par_bridge().map(process).sum()
    } else {
        current.map(process).sum()
    };

    finish_bar(&bar, t1_start.elapsed(), count, n);

    count
}

pub fn enumerate(opts: &EnumerateOpts) {
    let n = opts.n;
    let cache = !opts.no_cache;

    let start = Instant::now();

    let seed_list = if opts.no_cache {
        CacheOrbase::Base(false)
    } else {
        load_cache(n)
    };

    //Select enumeration function to run
    let cubes_len = match (opts.mode, opts.no_parallelism) {
        (EnumerationMode::Standard, no_parallelism) => {
            let cubes =
                unique_expansions(cache, n, opts.cache_compression, seed_list, !no_parallelism);
            cubes.len()
        }
        (EnumerationMode::RotationReduced, not_parallel) => {
            if n > 16 {
                println!("n > 16 not supported for rotation reduced");
                return;
            }
            if !not_parallel {
                println!("no parallel implementation for rotation-reduced, running single threaded")
            }
            let bar = if let (_, Some(max)) = seed_list.size_hint() {
                make_bar(max as u64)
            } else {
                unknown_bar()
            };

            rotation_reduced::gen_polycubes(n, &bar)
        }
        (EnumerationMode::PointList, not_parallel) => {
            if n > 16 {
                println!("n > 16 not supported for point-list");
                return;
            }
            let bar = if let (_, Some(max)) = seed_list.size_hint() {
                make_bar(max as u64)
            } else {
                unknown_bar()
            };

            let startn = seed_list.n() + 1;
            let cubes = pointlist::gen_polycubes(
                n,
                cache,
                !not_parallel,
                seed_list.collect(),
                startn,
                &bar,
            );
            cubes.len()
        }
        (EnumerationMode::Hashless, not_parallel) => {
            enumerate_hashless(n, !not_parallel, seed_list)
        }
    };

    let duration = start.elapsed();

    println!("Unique polycubes found for N = {n}: {cubes_len}.",);
    println!("Duration: {} ms", duration.as_millis());
}
