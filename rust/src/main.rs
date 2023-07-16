use std::{
    collections::HashSet,
    io::ErrorKind,
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
    /// Enumerate polycubes with a specific amount of cubes present
    Enumerate(EnumerateOpts),
    /// Validate the contents of a pcube file
    Validate(ValidateArgs),
}

#[derive(Clone, Args)]
pub struct ValidateArgs {
    /// The path of the PCube file to check
    pub path: String,

    /// Don't validate that all polycubes in the file are unique
    #[clap(short = 'u', long)]
    pub no_uniqueness: bool,

    /// Don't validate that all of the cubes in the file are canonical if
    /// the file header indicates that they should be
    #[clap(short = 'c', long)]
    pub no_canonical: bool,

    /// Validate that all polycubes in the file have exactly N
    /// cubes present
    #[clap(long, short)]
    pub n: Option<usize>,
}

#[derive(Clone, Args)]
pub struct EnumerateOpts {
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
            for n in (calculate_from..=n).rev() {
                let name = format!("cubes_{n}.pcube");
                let cache = match PolyCubeFile::new(&name) {
                    Ok(c) => c,
                    Err(e) => {
                        if e.kind() == ErrorKind::InvalidData || e.kind() == ErrorKind::Other {
                            println!("Enountered invalid cache file {name}. Error: {e}.");
                        }
                        continue;
                    }
                };

                println!("Found cache for N = {n}. Loading data...");

                if !cache.canonical() {
                    println!("Cached cubes are not canonical. Canonicalizing...")
                }

                let len = cache.len();
                calculate_from = n + 1;

                let mut error = None;
                let mut total_loaded = 0;
                let cached: HashSet<_> = cache
                    .filter_map(|v| {
                        total_loaded += 1;
                        match v {
                            Ok(v) => Some(v),
                            Err(e) => {
                                error = Some(e);
                                None
                            }
                        }
                    })
                    .collect();

                let total_len = len.unwrap_or(total_loaded);

                if total_len != cached.len() {
                    println!("There were non-unique cubes in the cache file. Continuing...")
                }

                if let Some(e) = error {
                    println!("Error occured while loading {name}. Error: {e}");
                } else {
                    current = cached.into_iter().collect();
                    break;
                }
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

pub fn validate(opts: &ValidateArgs) -> std::io::Result<()> {
    let path = &opts.path;
    let uniqueness = !opts.no_uniqueness;
    let validate_canonical = !opts.no_canonical;

    let mut file = PolyCubeFile::new(path)?;
    file.should_canonicalize = false;
    let canonical = file.canonical();

    println!("Validating {}", path);

    let read: Vec<_> = file.collect();

    if let Some(e) = read.iter().find_map(|r| r.as_ref().err()) {
        eprintln!("Error: Reading the file failed. Error: {e}.");
        std::process::exit(1);
    }

    let success: Vec<_> = read.into_iter().filter_map(|v| v.ok()).collect();

    println!("Read {} cubes from file succesfully.", success.len());

    if canonical && validate_canonical {
        println!("Verifying that provided file is canonical (header indicates that it is)...");

        if let Some(_) = success.iter().find(|v| {
            v != &&v
                .all_rotations()
                .max_by(PolyCube::canonical_ordering)
                .unwrap()
        }) {
            eprintln!(
                "Error: Found non-canonical polycube in file that claims to contain canonical cubes."
            );
            std::process::exit(1);
        }
    }

    if let Some(n) = opts.n {
        println!("Verifying that all polycubes in the file are N = {n}...");

        if let Some(v) = success.iter().find_map(|v| {
            if v.present_cubes() != n {
                Some(v.present_cubes())
            } else {
                None
            }
        }) {
            eprintln!("Error: Found a cube with N != {n}. Value: {v}");
            std::process::exit(1);
        }
    }

    if uniqueness {
        println!("Verifying that all polycubes in the file are unique...");

        // PolyCubeFile always spits out canonicalized polycubes
        //
        // TODO: typestate?
        let canonicalized = PolyCubeFile::new(path).unwrap();

        let unique: HashSet<_> = canonicalized.map(|v| v.ok()).collect();

        if unique.len() != success.len() {
            eprintln!(
                "Unique polycubes: {}. Total polycubes: {}. File contains multiple occurences some polycubes",
                unique.len(),
                success.len()
            );
            std::process::exit(1);
        }
    }

    println!("Validation succesful");

    Ok(())
}

pub fn enumerate(opts: &EnumerateOpts) {
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
        Opts::Enumerate(r) => enumerate(&r),
        Opts::Validate(a) => validate(&a).unwrap(),
    }
}
