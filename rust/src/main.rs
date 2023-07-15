use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use polycubes::PolyCube;

fn unique_expansions(alloc_tracker: Arc<AtomicUsize>, n: usize) -> Vec<PolyCube> {
    if n == 0 {
        return Vec::new();
    }

    let mut base = PolyCube::new_with_alloc_count(alloc_tracker.clone(), 1, 1, 1);

    base.set(0, 0, 0).unwrap();

    let mut current = [base].to_vec();

    if n > 1 {
        for i in 0..n - 1 {
            let next = PolyCube::unique_expansions(i + 2, current.iter());

            current = next;
        }
    }

    current
}

fn main() {
    let count = match std::env::args().skip(1).next() {
        Some(count) => count,
        None => {
            eprintln!("Missing `count` argument.");
            std::process::exit(1);
        }
    };

    let count: usize = if let Ok(v) = count.parse() {
        v
    } else {
        eprintln!("Invalid value for `count` argument.");
        std::process::exit(1);
    };

    let alloc_tracker = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();

    let cubes = unique_expansions(alloc_tracker.clone(), count);

    let duration = start.elapsed();

    let cubes = cubes.len();
    let allocations = alloc_tracker.load(Ordering::Relaxed);

    println!("Unique polycubes found for N = {count}: {cubes}, Total allocations: {allocations}",);
    println!("Duration: {} ms", duration.as_millis());
}
