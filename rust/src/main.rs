use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use clap::{Args, Parser};
use polycubes::{PolyCube, PolyCubeFile};

#[derive(Clone, Parser)]
pub enum Opts {
    Run(RunOpts),
    /// Validate the contents of a pcube file.
    Validate(ValidateArgs),
}

#[derive(Clone, Args)]
pub struct ValidateArgs {
    /// The path of the PCube file to check
    pub path: String,
    /// Validate that all values in the file are unique
    #[clap(short, long)]
    pub uniqueness: bool,
}

#[derive(Clone, Args)]
pub struct RunOpts {
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
                if let Ok(cache_file) = PolyCubeFile::new(format!("cubes_{}.pcube", i)) {
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
            }
        }

        for i in calculate_from..=n {
            println!("Calculating for N = {i}");
            let next = expansion_fn(i, current.iter());

            if use_cache {
                let name = &format!("cubes_{i}.pcube");
                if !std::fs::File::open(name).is_ok() {
                    println!("Saving {} to cache file", next.len());
                    PolyCubeFile::write(next.iter(), true, std::fs::File::create(name).unwrap())
                        .unwrap();
                }
            }

            current = next;
        }
    }

    current
}

pub fn validate(path: String) -> std::io::Result<()> {
    let mut file = PolyCubeFile::new(&path)?;
    file.should_canonicalize = false;
    let canonical = file.canonical();

    println!("Validating {}", path);

    let read: Vec<_> = file.collect();

    if let Some(e) = read.iter().find_map(|r| r.as_ref().err()) {
        panic!("Reading the file failed. Error: {e}.");
    }

    let success: Vec<_> = read.into_iter().filter_map(|v| v.ok()).collect();

    println!("Read {} cubes from file succesfully.", success.len());

    if canonical {
        println!("Header says that cubes are canonical. Verifying...");

        if let Some(_) = success.iter().find(|v| {
            v != &&v
                .all_rotations()
                .max_by(PolyCube::canonical_ordering)
                .unwrap()
        }) {
            panic!("Found non-canonical polycube in file that claims to contain canonical cubes.");
        }
    }

    println!("Validation succesful");

    Ok(())
}

pub fn run(opts: &RunOpts) {
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

fn main() {
    let opts = Opts::parse();

    match opts {
        Opts::Run(r) => run(&r),
        Opts::Validate(a) => validate(a.path).unwrap(),
    }
}
