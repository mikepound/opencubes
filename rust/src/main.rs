use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use clap::Parser;
use polycubes::PolyCube;

#[derive(Clone, Parser)]
pub struct Opts {
    /// The N value for which to calculate all unique polycubes.
    pub n: usize,

    /// Disable parallelism.
    #[clap(long, short = 'p')]
    pub no_parallelism: bool,
}

fn unique_expansions<F>(
    mut expansion_fn: F,
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
        for i in 0..n - 1 {
            let next = expansion_fn(i + 2, current.iter());

            current = next;
        }
    }

    current
}

fn main() {
    let opts = Opts::parse();

    let n = opts.n;

    let alloc_tracker = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let cubes = if opts.no_parallelism {
        unique_expansions(
            |n, current: std::slice::Iter<'_, PolyCube>| {
                PolyCube::unique_expansions(true, n, current)
            },
            alloc_tracker.clone(),
            n,
        )
    } else {
        unique_expansions(
            |n, current: std::slice::Iter<'_, PolyCube>| {
                PolyCube::unique_expansions_rayon(true, n, current)
            },
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
