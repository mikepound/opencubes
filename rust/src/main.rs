use std::{
    collections::HashSet,
    io::ErrorKind,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use clap::{Args, Parser};
use indicatif::{ProgressBar, ProgressStyle};
use polycubes::{make_bar, PolyCube, PolyCubeFile};

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

    /// Don't attempt to read the file into memory.
    ///
    /// If this flag is enabled, uniqueness cannot be checked.
    #[clap(long, short = 'm')]
    pub no_in_memory: bool,
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
                    println!("Saving {} cubes to cache file", next.len());
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
    let in_memory = !opts.no_in_memory;
    let n = opts.n;

    println!("Validating {}", path);

    let mut uniqueness = match (in_memory, uniqueness) {
        (true, true) => {
            eprintln!("Verifying uniqueness.");
            Some(HashSet::new())
        }
        (false, true) => {
            println!("Cannot verify uniqueness without placing all entries in memory. Re-run with `--no-uniqueness` enabled to run.");
            std::process::exit(1);
        }
        (_, false) => {
            eprintln!("Not verifying uniqueness");
            None
        }
    };

    let mut file = PolyCubeFile::new(path)?;
    file.should_canonicalize = false;
    let canonical = file.canonical();
    let len = file.len();

    let bar = if let Some(len) = len {
        make_bar(len as u64)
    } else {
        let style =
            ProgressStyle::with_template("[{elapsed_precise}] [{spinner:10.cyan/blue}] {msg}")
                .unwrap()
                .tick_strings(&[
                    "=>--------",
                    "<=>-------",
                    "-<=>------",
                    "--<=>-----",
                    "---<=>----",
                    "----<=>---",
                    "-----<=>--",
                    "------<=>-",
                    "-------<=>",
                    "--------<=",
                    "-------<=>",
                    "------<=>-",
                    "----<=>---",
                    "-----<=>--",
                    "---<=>----",
                    "--<=>-----",
                    "-<=>------",
                    "<=>-------",
                    "=>--------",
                    "----------",
                ]);

        ProgressBar::new(100).with_style(style)
    };

    let exit = |msg: &str| {
        bar.finish();
        println!("{msg}");
        std::process::exit(1);
    };

    match (canonical, validate_canonical) {
        (true, true) => eprintln!("Verifying entry canonicality. File indicates that entries are canonical."),
        (false, true) => eprintln!("Not verifying entry canonicality. File header does not indicate that entries are canonical"),
        (true, false) => eprintln!("Not verifying entry canonicality. File header indicates that they are, but check is disabled."),
        (false, false) => eprintln!("Not verifying canonicality. File header does not indicate that entries are canonical, and check is disabled.")
    }

    if let Some(n) = n {
        eprintln!("Verifying that all entries are N = {n}");
    }

    let mut total_read = 0;

    let mut last_tick = Instant::now();
    bar.tick();

    for cube in file {
        let cube = match cube {
            Ok(c) => c,
            Err(e) => {
                println!("Error: Reading the file failed. Error: {e}.");
                std::process::exit(1);
            }
        };

        total_read += 1;

        if len.is_some() {
            bar.inc(1);
            bar.tick();
        } else if last_tick.elapsed() >= Duration::from_millis(66) {
            last_tick = Instant::now();
            bar.set_message(format!("{total_read}"));
            bar.inc(1);
            bar.tick();
        }

        let mut form: Option<PolyCube> = None;
        let canonical_form = || {
            cube.all_rotations()
                .max_by(PolyCube::canonical_ordering)
                .unwrap()
        };

        if canonical && validate_canonical {
            if form.get_or_insert_with(|| canonical_form()) != &cube {
                exit(
                    "Error: Found non-canonical polycube in file that claims to contain canonical cubes."
                );
            }
        }

        if let Some(n) = n {
            let v = cube.present_cubes();
            if v != n {
                exit(&format!("Error: Found a cube with N != {n}. Value: {v}"));
            }
        }

        if let Some(uniqueness) = &mut uniqueness {
            let form = form.get_or_insert_with(|| canonical_form()).clone();
            if !uniqueness.insert(form) {
                exit("Found non-unique polycubes.");
            }
        }
    }

    bar.finish();

    println!("Success: {path}, containing {total_read} cubes, is valid");

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
