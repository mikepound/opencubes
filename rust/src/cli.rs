use std::{
    collections::{BTreeMap, HashSet},
    io::ErrorKind,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use opencubes::{
    naive_polycube::NaivePolyCube,
    pcube::{PCubeFile, RawPCube},
};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

mod pointlist;
mod polycube_reps;
mod rotation_reduced;
mod rotations;

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

pub fn make_bar(len: u64) -> indicatif::ProgressBar {
    let bar = ProgressBar::new(len);

    let pos_width = format!("{len}").len();

    let template =
        format!("[{{elapsed_precise}}] {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} {{msg}}");

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
            } else {
                println!("Could not load cache file '{name}'. Error: {e}");
            }
            None
        }
    }
}

/// load closes cache file to n into a vec
/// returns a vec and the next order above the found cache file
fn load_cache(n: usize) -> (Vec<RawPCube>, usize) {
    let calculate_from = 2;

    for n in (calculate_from..n).rev() {
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

            return (cached.into_iter().collect(), n + 1);
        }
    }

    println!("no cache file found reverting to start building from n=1");
    let mut base = RawPCube::new_empty(1, 1, 1);
    base.set(0, 0, 0, true);

    let current = [base.clone()].to_vec();
    //calculate from 2 because 1 is in the vec
    (current, 2)
}

fn unique_expansions<F>(
    mut expansion_fn: F,
    use_cache: bool,
    n: usize,
    compression: Compression,
    current: Vec<RawPCube>,
    calculate_from: usize,
) -> Vec<NaivePolyCube>
where
    F: FnMut(&ProgressBar, std::slice::Iter<'_, NaivePolyCube>) -> Vec<NaivePolyCube>,
{
    if n == 0 {
        return Vec::new();
    }

    let mut current = current
        .into_iter()
        .map(NaivePolyCube::from)
        .map(|v| v.canonical_form())
        .collect::<Vec<_>>();

    for i in calculate_from..=n {
        let bar = make_bar(current.len() as u64);
        bar.set_message(format!("base polycubes expanded for N = {i}..."));

        let start = Instant::now();

        let next = expansion_fn(&bar, current.iter());

        bar.set_message(format!(
            "Found {} unique expansions (N = {i}) in {} ms.",
            next.len(),
            start.elapsed().as_millis(),
        ));

        bar.finish();

        if use_cache {
            let name = &format!("cubes_{i}.pcube");
            if !std::fs::File::open(name).is_ok() {
                println!("Saving {} cubes to cache file", next.len());
                PCubeFile::write_file(false, compression.into(), next.iter().map(Into::into), name)
                    .unwrap();
            } else {
                println!("Cache file already exists for N = {i}. Not overwriting.");
            }
        }

        current = next;
    }

    current
}

pub fn enumerate(opts: &EnumerateOpts) {
    let n = opts.n;
    let cache = !opts.no_cache;

    if n < 2 {
        println!("n < 2 unsuported");
        return;
    }

    let start = Instant::now();

    let (seed_list, startn) = if cache {
        load_cache(n)
    } else {
        let mut base = RawPCube::new_empty(1, 1, 1);
        base.set(0, 0, 0, true);

        let current = [base].to_vec();
        //calculate from 2 because 1 is in the vec
        (current, 2)
    };

    //Select enumeration function to run
    let cubes_len = match (opts.mode, opts.no_parallelism) {
        (EnumerationMode::Standard, true) => {
            let cubes = unique_expansions(
                |bar, current: std::slice::Iter<'_, NaivePolyCube>| {
                    NaivePolyCube::unique_expansions(bar, current)
                },
                cache,
                n,
                opts.cache_compression,
                seed_list,
                startn,
            );
            cubes.len()
        }
        (EnumerationMode::Standard, false) => {
            let cubes = unique_expansions(
                |bar, current: std::slice::Iter<'_, NaivePolyCube>| {
                    NaivePolyCube::unique_expansions_rayon(bar, current)
                },
                cache,
                n,
                opts.cache_compression,
                seed_list,
                startn,
            );
            cubes.len()
        }
        (EnumerationMode::RotationReduced, para) => {
            if n > 16 {
                println!("n > 16 not supported for rotation reduced");
                return;
            }
            if !para {
                println!("no parallel implementation for rotation-reduced, running single threaded")
            }
            rotation_reduced::gen_polycubes(n)
        }
        (EnumerationMode::PointList, para) => {
            if n > 16 {
                println!("n > 16 not supported for point-list");
                return;
            }
            let cubes = pointlist::gen_polycubes(
                n,
                cache,
                opts.cache_compression,
                !para,
                seed_list,
                startn,
            );
            cubes.len()
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
