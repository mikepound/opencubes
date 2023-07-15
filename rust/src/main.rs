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

    println!("Calculating unique polycubes for N = 1");

    let mut base = PolyCube::new_with_alloc_count(alloc_tracker.clone(), 1, 1, 1);

    base.set(0, 0, 0).unwrap();

    let mut current = [base].to_vec();

    if n > 1 {
        for i in 0..n - 1 {
            println!("Calculating unique polycubes for N = {}", i + 2);

            let start = Instant::now();

            let next = PolyCube::unique_expansions(current.iter());

            let duration = start.elapsed();

            println!(
                "Took {} ms to calculate polycubes for N = {}. Unique polycubes: {}",
                duration.as_millis(),
                i + 2,
                next.len(),
            );

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

    println!("Unique polycubes found: {cubes}, Total allocations: {allocations}",);
    println!("Duration: {} ms", duration.as_millis());
}
