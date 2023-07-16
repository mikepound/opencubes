use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use clap::Parser;
use polycubes::{PolyCube, PolyCubeFileReader};

#[derive(Clone, Parser)]
pub struct Opts {
    /// The N value for which to calculate all unique polycubes.
    pub n: usize,

    /// Disable parallelism.
    #[clap(long, short = 'p')]
    pub no_parallelism: bool,

    /// Don't use the cache
    #[clap(long, short = 'c')]
    pub no_cache: bool,
}

fn unique_expansions<F>(
    mut expansion_fn: F,
    use_cache: bool,
    alloc_tracker: Arc<AtomicUsize>,
    n: usize,
) -> Vec<PolyCube>
where
    F: FnMut(usize, std::slice::Iter<'_, PolyCube>) -> Vec<PolyCube>,
{
    if n == 0 {
        return Vec::new();
    }

    let mut base = PolyCube::new_with_alloc_count(alloc_tracker.clone(), 1, 1, 1);

    base.set(0, 0, 0).unwrap();

    let mut current = [base].to_vec();

    if n > 1 {
        let mut calculate_from = 2;

        if use_cache {
            let mut highest = None;
            for i in calculate_from..=n {
                if let Ok(cache_file) = PolyCubeFileReader::new(format!("cubes_{}.pcube", i)) {
                    highest = Some((i, cache_file));
                }
            }

            if let Some((n, cache)) = highest {
                println!("Found cache for N = {n}. Loading data...");

                if !cache.canonical() {
                    println!("Cached cubes are not canonical. Canonicalizing...")
                }

                let len = cache.len();
                calculate_from = n + 1;
                let cached: HashSet<_> = cache
                    .filter_map(|v| match v {
                        Ok(v) => Some(v),
                        Err(e) => panic!("Failed to load a cube. {e}"),
                    })
                    .collect();

                if let Some(len) = len {
                    assert_eq!(
                        len,
                        cached.len(),
                        "There were non-unique cubes in the cache."
                    );
                } else {
                    panic!("Cannot determine if all cubes in the cache where unique.");
                }

                current = cached.into_iter().collect();

                println!("Done!");
            }
        }

        for i in calculate_from..=n {
            let next = expansion_fn(i, current.iter());

            if use_cache {
                let name = &format!("cubes_{i}.pcube");
                if !std::fs::File::open(name).is_ok() {
                    println!("Saving data to cache");
                }
            }

            current = next;
        }
    }

    current
}

#[allow(unreachable_code, unused)]
fn main() {
    let opts = Opts::parse();

    let n = opts.n;
    let cache = !opts.no_cache;

    let alloc_tracker = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let cubes = if opts.no_parallelism {
        unique_expansions(
            |n, current: std::slice::Iter<'_, PolyCube>| {
                PolyCube::unique_expansions(true, n, current)
            },
            cache,
            alloc_tracker.clone(),
            n,
        )
    } else {
        unique_expansions(
            |n, current: std::slice::Iter<'_, PolyCube>| {
                PolyCube::unique_expansions_rayon(true, n, current)
            },
            cache,
            alloc_tracker.clone(),
            n,
        )
    };

    let duration = start.elapsed();

    let cubes = cubes.len();
    let allocations = alloc_tracker.load(Ordering::Relaxed);

    println!("Unique polycubes found for N = {n}: {cubes}, Total allocations: {allocations}",);
    println!("Duration: {} ms", duration.as_millis());
}
