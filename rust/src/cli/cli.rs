use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    time::Duration,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use opencubes::{naive_polycube::NaivePolyCube, pcube::PCubeFile};

mod enumerate;
use enumerate::enumerate;
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

fn finish_bar(bar: &ProgressBar, duration: Duration, expansions: usize, n: usize) {
    let time = duration.as_micros();
    let secs = time / 1_000_000;
    let micros = time % 1_000_000;

    if let Some(len) = bar.length() {
        let pos_width = format!("{}", len).len();

        let template = format!(
            "[{{elapsed_precise}}] {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} {{msg}}"
        );

        bar.set_style(
            ProgressStyle::with_template(&template)
                .unwrap()
                .progress_chars("#>-"),
        );
    }

    bar.finish_with_message(format!(
        "Done! Found {expansions} expansions (N = {n}) in {secs}.{micros} s"
    ));
}

fn unknown_bar() -> ProgressBar {
    unknown_bar_with_pos(false)
}

fn unknown_bar_with_pos(with_pos: bool) -> ProgressBar {
    let template = if with_pos {
        "[{elapsed_precise}] [{spinner:10.cyan/blue}] {pos} {msg}"
    } else {
        "[{elapsed_precise}] [{spinner:10.cyan/blue}] {msg}"
    };

    let style = ProgressStyle::with_template(template)
        .unwrap()
        .tick_strings(&[
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
        format!("[{{elapsed_precise}}] {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} {{msg}} remaining: [{{eta_precise}}]");

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

    let file = PCubeFile::new_file(path)?;
    let canonical = file.canonical();
    let len = file.len();

    let bar = if let Some(len) = len {
        make_bar(len as u64)
    } else {
        unknown_bar_with_pos(true)
    };

    bar.set_message("cubes validated");

    bar.println(format!("Validating {}", path));

    let mut uniqueness = match (in_memory, uniqueness) {
        (true, true) => {
            bar.println("Verifying uniqueness.");
            Some(HashSet::new())
        }
        (false, true) => {
            bar.abandon();
            println!("Cannot verify uniqueness without placing all entries in memory. Re-run with `--no-uniqueness` enabled to run.");
            std::process::exit(1);
        }
        (_, false) => {
            bar.println("Not verifying uniqueness");
            None
        }
    };

    let exit = |msg: &str| {
        bar.abandon();
        println!("{msg}");
        std::process::exit(1);
    };

    match (canonical, validate_canonical) {
        (true, true) => bar.println("Verifying entry canonicality. File indicates that entries are canonical."),
        (false, true) => bar.println("Not verifying entry canonicality. File header does not indicate that entries are canonical"),
        (true, false) => bar.println("Not verifying entry canonicality. File header indicates that they are, but check is disabled."),
        (false, false) => bar.println("Not verifying canonicality. File header does not indicate that entries are canonical, and check is disabled.")
    }

    if let Some(n) = n {
        bar.println(format!("Verifying that all entries are N = {n}"));
    }

    let mut total_read = 0;

    for cube in file {
        let cube = match cube {
            Ok(c) => NaivePolyCube::from(c),
            Err(e) => {
                println!("Error: Reading the file failed. Error: {e}.");
                std::process::exit(1);
            }
        };

        total_read += 1;

        bar.inc(1);

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
    }

    bar.finish();

    println!("Success: {path}, containing {total_read} cubes, is valid");

    Ok(())
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
        .par_iter()
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
        .map(|(len, (input_file, path))| {
            let output_path = opts.output_path.clone().unwrap_or(path.clone());

            multi_bar
                .println(format!("Converting file {}", path))
                .unwrap();
            multi_bar
                .println(format!("Final output path: {output_path}"))
                .unwrap();

            if opts.canonicalize {
                multi_bar.println("Canonicalizing output").unwrap();
            }
            multi_bar
                .println(format!("Input compression: {:?}", input_file.compression()))
                .unwrap();
            multi_bar
                .println(format!("Output compression: {:?}", opts.compression))
                .unwrap();

            let bar = if let Some(len) = len {
                make_bar(len as u64)
            } else {
                unknown_bar_with_pos(true)
            };

            let bar = multi_bar.add(bar);

            (input_file, path, output_path, bar)
        })
        .collect();

    // Convert, in parallel
    files
        .into_par_iter()
        .for_each(|(input_file, path, output_path, bar)| {
            bar.set_message(format!("cubes converted for {path}"));

            let canonical = input_file.canonical();
            let mut output_path_temp = PathBuf::from(&output_path);
            let filename = output_path_temp.file_name().unwrap();
            let filename = filename.to_string_lossy().to_string();
            let filename = format!(".{filename}.tmp");
            output_path_temp.pop();
            output_path_temp.push(filename);

            let input = input_file.filter_map(|v| {
                let cube = match v {
                    Ok(v) => Some(v),
                    Err(e) => {
                        let msg = format!("{path} Failed. Error: {e}");
                        bar.abandon_with_message(msg);
                        return None;
                    }
                }?;

                bar.inc(1);

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
                    Ok(_) => bar.finish(),
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
