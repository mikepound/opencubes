use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use polycubes::PolyCube;

fn unique_expansions(alloc_tracker: Rc<AtomicUsize>, n: usize) -> Vec<PolyCube> {
    if n == 0 {
        return Vec::new();
    }

    let mut base = PolyCube::new_with_alloc_count(alloc_tracker.clone(), 1, 1, 1);

    base.set(0, 0, 0).unwrap();

    let mut current = [base].to_vec();

    if n > 1 {
        for i in 0..n - 1 {
            let next = PolyCube::unique_expansions(current.iter());

            next.iter()
                .for_each(|v| assert_eq!(v.cube_iter().filter(|v| **v).count(), i + 2));

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

    let alloc_tracker = Rc::new(AtomicUsize::new(0));
    let l4 = unique_expansions(alloc_tracker.clone(), count);

    println!(
        "Unique polycubes found: {}, Total allocations: {}",
        l4.len(),
        alloc_tracker.load(Ordering::Relaxed)
    );
}
