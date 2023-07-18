use std::{
    collections::HashSet,
    io::ErrorKind,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use opencubes::{make_bar, PolyCube, PolyCubeFile};

fn unknown_bar() -> ProgressBar {
    let style = ProgressStyle::with_template("[{elapsed_precise}] [{spinner:10.cyan/blue}] {msg}")
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
}

#[derive(Clone, Parser)]
pub enum Opts {
    /// Enumerate polycubes with a specific amount of cubes present
    Enumerate(EnumerateOpts),
    /// Perform operations on pcube files
    #[clap(subcommand)]
    Pcube(PcubeCommands),
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
    pub path: String,

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

impl From<Compression> for opencubes::Compression {
    fn from(value: Compression) -> Self {
        match value {
            Compression::None => opencubes::Compression::None,
            Compression::Gzip => opencubes::Compression::Gzip,
        }
    }
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
        unknown_bar()
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

fn load_cache_file(n: usize) -> Option<PolyCubeFile> {
    let name = format!("cubes_{n}.pcube");

    match PolyCubeFile::new(&name) {
        Ok(file) => Some(file),
        Err(e) => {
            if e.kind() == ErrorKind::InvalidData || e.kind() == ErrorKind::Other {
                println!("Enountered invalid cache file {name}. Error: {e}.");
            } else {
                println!("Could not load cache file '{name}'. Error: {e}");
            }
            None
        }
    }
}

fn unique_expansions<F>(
    mut expansion_fn: F,
    use_cache: bool,
    n: usize,
    compression: Compression,
) -> Vec<PolyCube>
where
    F: FnMut(usize, std::slice::Iter<'_, PolyCube>) -> Vec<PolyCube>,
{
    if n == 0 {
        return Vec::new();
    }

    let mut base = PolyCube::new(1, 1, 1);

    base.set(0, 0, 0).unwrap();

    let mut current = [base].to_vec();

    if n > 1 {
        let mut calculate_from = 2;

        if use_cache {
            for n in (calculate_from..=n).rev() {
                let name = format!("cubes_{n}.pcube");
                let cache = if let Some(file) = load_cache_file(n) {
                    file
                } else {
                    continue;
                };

                println!("Found cache for N = {n}. Loading data...");

                if !cache.canonical() {
                    println!("Cached cubes are not canonical. Canonicalizing...")
                }

                let len = cache.len();
                calculate_from = n + 1;

                let mut error = None;
                let mut total_loaded = 0;

                let filter = |value| {
                    total_loaded += 1;
                    match value {
                        Ok(v) => Some(v),
                        Err(e) => {
                            error = Some(e);
                            None
                        }
                    }
                };

                let cached: HashSet<_> = cache.filter_map(filter).collect();

                if let Some(e) = error {
                    println!("Error occured while loading {name}. Error: {e}");
                } else {
                    let total_len = len.unwrap_or(total_loaded);

                    if total_len != cached.len() {
                        println!("There were non-unique cubes in the cache file. Continuing...")
                    }

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
                    PolyCubeFile::write(
                        next.iter(),
                        true,
                        compression.into(),
                        std::fs::File::create(name).unwrap(),
                    )
                    .unwrap();
                }
            }

            current = next;
        }
    }

    current
}

pub fn enumerate(opts: &EnumerateOpts) {
    let n = opts.n;
    let cache = !opts.no_cache;

    let start = Instant::now();

    let cubes = if opts.no_parallelism {
        unique_expansions(
            |n, current: std::slice::Iter<'_, PolyCube>| {
                PolyCube::unique_expansions(true, n, current)
            },
            cache,
            n,
            opts.cache_compression,
        )
    } else {
        unique_expansions(
            |n, current: std::slice::Iter<'_, PolyCube>| {
                PolyCube::unique_expansions_rayon(true, n, current)
            },
            cache,
            n,
            opts.cache_compression,
        )
    };

    let duration = start.elapsed();

    let cubes = cubes.len();

    println!("Unique polycubes found for N = {n}: {cubes}.",);
    println!("Duration: {} ms", duration.as_millis());
}

pub fn convert(opts: &ConvertArgs) {
    let output_path = opts.output_path.as_ref().unwrap_or(&opts.path);

    println!("Converting file {}", opts.path);
    println!("Final output path: {output_path}");

    let mut input_file = match PolyCubeFile::new(&opts.path) {
        Ok(f) => f,
        Err(e) => {
            println!("Failed to open input file. Error: {e}");
            std::process::exit(1);
        }
    };

    input_file.should_canonicalize = opts.canonicalize;

    if opts.canonicalize {
        println!("Canonicalizing output");
    }
    println!("Input compression: {:?}", input_file.compression());
    println!("Output compression: {:?}", opts.compression);

    let canonical = input_file.canonical();
    let len = input_file.len();

    let bar = if let Some(len) = len {
        make_bar(len as u64)
    } else {
        unknown_bar()
    };

    let exit = |msg: &str| -> ! {
        bar.finish();
        eprintln!("{msg}");
        std::process::exit(1);
    };

    let mut output_path_temp = PathBuf::from(output_path);
    let filename = output_path_temp.file_name().unwrap();
    let filename = filename.to_string_lossy().to_string();
    let filename = format!(".{filename}.tmp");
    output_path_temp.pop();
    output_path_temp.push(filename);

    let output_file = match std::fs::File::create(&output_path_temp) {
        Ok(f) => f,
        Err(e) => exit(&format!("Failed to create temporary output file. {e}")),
    };

    let mut total_read = 0;
    let mut last_tick = Instant::now();
    bar.tick();

    let input = input_file.filter_map(|v| {
        total_read += 1;

        if len.is_some() {
            bar.inc(1);
        } else if last_tick.elapsed() >= Duration::from_millis(66) {
            last_tick = Instant::now();
            bar.set_message(format!("{total_read}"));
            bar.inc(1);
            bar.tick();
        }

        match v {
            Ok(v) => Some(v),
            Err(e) => exit(&format!(
                "Failed to read all cubes from input file. Error: {e}"
            )),
        }
    });

    match PolyCubeFile::write(input, canonical, opts.compression.into(), output_file) {
        Ok(_) => {}
        Err(e) => exit(&format!("Failed. Error: {e}.")),
    }

    bar.finish();

    match std::fs::rename(output_path_temp, output_path) {
        Ok(_) => println!("Success"),
        Err(e) => exit(&format!("Failed to write final file: {e}")),
    }
}

fn info(path: &str) {
    let file = match PolyCubeFile::new(path) {
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
