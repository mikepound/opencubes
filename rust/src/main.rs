use std::{
    collections::HashSet,
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
    let alloc_tracker = Rc::new(AtomicUsize::new(0));

    let l4 = unique_expansions(alloc_tracker.clone(), 9);

    let l4_hash: HashSet<_> = l4.iter().map(Clone::clone).collect();

    println!("{}, {}", l4.len(), alloc_tracker.load(Ordering::Relaxed));

    l4.iter().skip(117).take(2).for_each(|v| println!("{v}"));
    l4.iter().skip(159).take(2).for_each(|v| println!("{v}"));

    assert_eq!(l4_hash.len(), l4.len(), "Hash disagrees with output list");
    assert_eq!(l4.len(), 29, "Incorrect length");
}
