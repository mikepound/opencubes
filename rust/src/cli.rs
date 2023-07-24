use std::{
    collections::{BTreeMap, HashSet},
    io::ErrorKind,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use opencubes::{
    hashless,
    iterator::{
        indicatif::PolycubeProgressBarIter, AllPolycubeIterator, AllUniquePolycubeIterator,
        PolycubeIterator, UniquePolycubeIterator,
    },
    naive_polycube::NaivePolyCube,
    pcube::{PCubeFile, RawPCube},
    pointlist, rotation_reduced,
};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

fn unknown_bar() -> ProgressBar {
    let style = ProgressStyle::with_template("[{elapsed_precise}] [{spinner:10.cyan/blue}] {msg}")
        .unwrap()
        .tick_strings(&[
            ">---------",
            ">---------",
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
            "---------<",
            "--------<=",
            "-------<=>",
            "------<=>-",
            "-----<=>--",
            "---<=>----",
            "--<=>-----",
            "-<=>------",
            "<=>-------",
            "=>--------",
        ]);

    let bar = ProgressBar::new(100).with_style(style);

    bar.enable_steady_tick(Duration::from_millis(66));

    bar
}

pub fn make_bar(len: u64) -> indicatif::ProgressBar {
    let bar = ProgressBar::new(len);

    let pos_width = format!("{len}").len();

    let template =
        format!("[{{elapsed_precise}}] {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} remaining: [{{eta_precise}}] {{msg}}");

    bar.set_style(
        ProgressStyle::with_template(&template)
            .unwrap()
            .progress_chars("#>-"),
    );
    bar
}

#[derive(Clone, Parser)]
pub enum Opts {
    /// Enumerate polycubes with a specific amount of cubes present
    Enumerate(EnumerateOpts),
    /// Perform operations on pcube files
    #[clap(subcommand)]
    Pcube(PcubeCommands),
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

    /// Compress written cache files
    #[clap(long, short = 'z', value_enum, default_value = "none")]
    pub cache_compression: Compression,

    #[clap(long, short = 'm', value_enum, default_value = "standard")]
    pub mode: EnumerationMode,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum EnumerationMode {
    Standard,
    RotationReduced,
    PointList,
    Hashless,
}

#[derive(Clone, Subcommand)]
pub enum PcubeCommands {
    Validate(ValidateArgs),
    Convert(ConvertArgs),
    Info {
        #[clap(required = true)]
        path: Vec<String>,
    },
}

#[derive(Clone, Args)]
pub struct ConvertArgs {
    /// The path of the pcube file to convert
    #[clap(required = true)]
    pub path: Vec<String>,

    /// The output compression to use
    #[clap(long, short = 'z', value_enum, default_value = "none")]
    pub compression: Compression,

    /// Canonicalize the input polycubes
    #[clap(long, short)]
    pub canonicalize: bool,

    /// The path to output the converted pcube file to.
    ///
    /// Defaults to `path`, overwriting the original once
    /// the conversion is complete.
    #[clap(short, long)]
    pub output_path: Option<String>,
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

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum Compression {
    None,
    Gzip,
}

impl From<Compression> for opencubes::pcube::Compression {
    fn from(value: Compression) -> Self {
        match value {
            Compression::None => opencubes::pcube::Compression::None,
            Compression::Gzip => opencubes::pcube::Compression::Gzip,
        }
    }
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

    let file = PCubeFile::new_file(path)?;
    let canonical = file.canonical();
    let len = file.len();

    let bar = if let Some(len) = len {
        make_bar(len as u64)
    } else {
        unknown_bar()
    };

    let exit = |msg: &str| {
        bar.abandon();
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
            Ok(c) => NaivePolyCube::from(c),
            Err(e) => {
                println!("Error: Reading the file failed. Error: {e}.");
                std::process::exit(1);
            }
        };

        total_read += 1;

        if len.is_some() {
            bar.inc(1);
        } else if last_tick.elapsed() >= Duration::from_millis(66) {
            last_tick = Instant::now();
            bar.set_message(format!("{total_read}"));
            bar.inc(1);
            bar.tick();
        }

        let mut form: Option<NaivePolyCube> = None;
        let canonical_form = || cube.pcube_canonical_form();

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

        bar.finish();
    }

    println!("Success: {path}, containing {total_read} cubes, is valid");

    Ok(())
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
        // let bar = make_bar(current.len() as u64);
        let bar = unknown_bar();
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

pub fn convert(opts: &ConvertArgs) {
    if opts.output_path.is_some() && opts.path.len() > 1 {
        println!("Cannot convert more than 1 file when output path is provided");
        std::process::exit(1);
    }

    let multi_bar = MultiProgress::new();

    // First create the files and put all of these into a BTreeMap so
    // that the longest files are yielded last.
    let files: BTreeMap<_, _> = opts
        .path
        .iter()
        .map(|path| {
            let input_file = match PCubeFile::new_file(&path) {
                Ok(f) => f,
                Err(e) => {
                    println!("Failed to open input file {path}. Error: {e}");
                    std::process::exit(1);
                }
            };
            (input_file.len(), (input_file, path.to_string()))
        })
        .collect();

    // Iterate over the files and do some printing, in-order
    let files: Vec<_> = files
        .into_iter()
        .map(|(_, (input_file, path))| {
            let output_path = opts.output_path.clone().unwrap_or(path.clone());

            println!("Converting file {}", path);
            println!("Final output path: {output_path}");
            if opts.canonicalize {
                println!("Canonicalizing output");
            }
            println!("Input compression: {:?}", input_file.compression());
            println!("Output compression: {:?}", opts.compression);

            let len = input_file.len();

            let bar = if let Some(len) = len {
                make_bar(len as u64)
            } else {
                unknown_bar()
            };

            let bar = multi_bar.add(bar);

            (input_file, path, output_path, len, bar)
        })
        .collect();

    // Convert, in parallel
    files
        .into_par_iter()
        .for_each(|(input_file, path, output_path, len, bar)| {
            bar.set_message(path.to_string());

            let canonical = input_file.canonical();
            let mut output_path_temp = PathBuf::from(&output_path);
            let filename = output_path_temp.file_name().unwrap();
            let filename = filename.to_string_lossy().to_string();
            let filename = format!(".{filename}.tmp");
            output_path_temp.pop();
            output_path_temp.push(filename);

            let mut total_read = 0;
            let mut last_tick = Instant::now();

            let input = input_file.filter_map(|v| {
                total_read += 1;

                let cube = match v {
                    Ok(v) => Some(v),
                    Err(e) => {
                        let msg = format!("{path} Failed. Error: {e}");
                        bar.abandon_with_message(msg);
                        return None;
                    }
                }?;

                if len.is_some() {
                    bar.inc(1);
                } else if last_tick.elapsed() >= Duration::from_millis(66) {
                    last_tick = Instant::now();
                    bar.set_message(format!("{total_read}"));
                    bar.inc(1);
                    bar.tick();
                }

                if opts.canonicalize {
                    Some(NaivePolyCube::from(cube).canonical_form().into())
                } else {
                    Some(cube)
                }
            });

            let canonical = canonical || opts.canonicalize;

            match PCubeFile::write_file(
                canonical,
                opts.compression.into(),
                input,
                &output_path_temp,
            ) {
                Ok(_) => {}
                Err(e) => {
                    bar.abandon_with_message(format!("Failed. Error: {e}."));
                    return;
                }
            }

            if !bar.is_finished() {
                match std::fs::rename(output_path_temp, output_path) {
                    Ok(_) => bar.finish_with_message(format!("{path} Done!")),
                    Err(e) => {
                        bar.abandon_with_message(format!("{path} Failed to write final file: {e}"));
                        return;
                    }
                }
            }
        });
}

fn info(path: &str) {
    let file = match PCubeFile::new_file(path) {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to open file. {e}");
            std::process::exit(1);
        }
    };

    let len = file
        .len()
        .map(|v| format!("{v}"))
        .unwrap_or("Unknown (is a stream)".to_string());
    let compression = file.compression();
    let canonical = file.canonical().then(|| "yes").unwrap_or("no");

    println!();
    println!("Info for {path}");
    println!("Amount of polycubes: {len}");
    println!("Compression method: {compression:?}");
    println!("In canonical position: {canonical}");
}

fn main() {
    let opts = Opts::parse();

    match opts {
        Opts::Enumerate(r) => enumerate(&r),
        Opts::Pcube(PcubeCommands::Validate(a)) => validate(&a).unwrap(),
        Opts::Pcube(PcubeCommands::Convert(c)) => convert(&c),
        Opts::Pcube(PcubeCommands::Info { path }) => path.iter().map(String::as_str).for_each(info),
    }
}
