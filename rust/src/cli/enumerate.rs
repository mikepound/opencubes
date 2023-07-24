use std::{io::ErrorKind, sync::Arc, time::Instant};

use ::indicatif::ProgressBar;
use opencubes::{
    hashless,
    iterator::{indicatif::PolycubeProgressBarIter, *},
    naive_polycube::NaivePolyCube,
    pcube::{PCubeFile, RawPCube},
    pointlist, rotation_reduced,
};

use crate::{make_bar, unknown_bar, Compression, EnumerateOpts, EnumerationMode};

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
    bar: &ProgressBar,
    compression: Compression,
    n: usize,
    // Ideally, this would be `AllUniquePolycubeIterator` but it's
    // a bit unwieldy
    cubes: impl Iterator<Item = RawPCube> + ExactSizeIterator,
) {
    let name = &format!("cubes_{n}.pcube");
    if !std::fs::File::open(name).is_ok() {
        bar.println(format!("Saving {} cubes to cache file", cubes.len()));
        PCubeFile::write_file(false, compression.into(), cubes, name).unwrap();
    } else {
        bar.println(format!(
            "Cache file already exists for N = {n}. Not overwriting."
        ));
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

/// load largest findable cachefile with size <= n - 1 into a vec
/// returns a vec and the next order above the found cache file
fn load_cache(n: usize) -> impl AllUniquePolycubeIterator {
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
        n - 1
    );

    CacheOrbase::Base(false)
}

fn unique_expansions(
    use_cache: bool,
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
        bar.set_message(format!("base polycubes expanded for N = {i}..."));

        let start = Instant::now();

        let with_bar = PolycubeProgressBarIter::new(bar.clone(), current);
        let next: Vec<RawPCube> = if parallel {
            NaivePolyCube::unique_expansions_rayon(with_bar).collect()
        } else {
            NaivePolyCube::unique_expansions(with_bar).collect()
        };

        bar.set_message(format!(
            "Found {} unique expansions (N = {}) in {} ms.",
            next.len(),
            i + 1,
            start.elapsed().as_millis(),
        ));

        bar.finish();

        if use_cache {
            save_to_cache(&bar, compression, i + 1, next.iter().map(Clone::clone));
        }

        i += 1;

        if i == n {
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

pub fn enumerate(opts: &EnumerateOpts) {
    let n = opts.n;
    let cache = !opts.no_cache;

    if n < 2 {
        println!("n < 2 unsuported");
        return;
    }

    let start = Instant::now();

    let seed_list = load_cache(n);

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
            let startn = seed_list.n() + 1;
            let bar = if let (_, Some(max)) = seed_list.size_hint() {
                make_bar(max as u64)
            } else {
                unknown_bar()
            };

            hashless::gen_polycubes(n, !not_parallel, seed_list.collect(), startn, &bar)
        }
    };

    let duration = start.elapsed();

    println!("Unique polycubes found for N = {n}: {cubes_len}.",);
    println!("Duration: {} ms", duration.as_millis());
}
