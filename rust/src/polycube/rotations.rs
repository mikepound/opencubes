//! This module implements an iterator that yeidls all of the rotations
//! of a polycube.

use crate::PolyCube;

struct PlaneIterator {
    count: usize,
    plane: (usize, usize),
    base: PolyCube,
}

impl Iterator for PlaneIterator {
    type Item = PolyCube;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count <= 3 {
            let out = self.base.clone().rot90(self.count, self.plane);
            self.count += 1;
            Some(out)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = 4 - self.count;
        (left, Some(left))
    }
}

impl ExactSizeIterator for PlaneIterator {
    fn len(&self) -> usize {
        let (lower, upper) = self.size_hint();
        // Note: This assertion is overly defensive, but it checks the invariant
        // guaranteed by the trait. If this trait were rust-internal,
        // we could use debug_assert!; assert_eq! will check all Rust user
        // implementations too.
        assert_eq!(upper, Some(lower));
        lower
    }
}

impl PolyCube {
    /// Obtain an iterator yielding all rotations of `self` in `plane`.
    pub fn rotations_in_plane(
        self,
        plane: (usize, usize),
    ) -> impl Iterator<Item = PolyCube> + ExactSizeIterator {
        PlaneIterator {
            count: 0,
            plane,
            base: self,
        }
    }

    /// Obtain an iterator yielding all possible rotations of `self`
    pub fn all_rotations(&self) -> impl Iterator<Item = PolyCube> + '_ {
        const _0_1: (usize, usize) = (0, 1);
        const _1_2: (usize, usize) = (1, 2);
        const _0_2: (usize, usize) = (0, 2);

        let rots_in_native_plane = self.clone().rotations_in_plane(_1_2);

        #[rustfmt::skip]
        let rotation_settings = [
            (2, _0_2, _1_2),
            (1, _0_2, _0_1),
            (3, _0_2, _0_1),
            (1, _0_1, _0_2),
            (3, _0_1, _0_2),
        ];

        let all_others = rotation_settings
            .into_iter()
            .flat_map(move |(k, p, rots_in_p)| {
                self.clone().rot90(k, p).rotations_in_plane(rots_in_p)
            });

        struct AllRotationsIter<I>
        where
            I: Iterator<Item = PolyCube>,
        {
            inner: I,
            rotations_checked: usize,
        }

        impl<I> Iterator for AllRotationsIter<I>
        where
            I: Iterator<Item = PolyCube>,
        {
            type Item = PolyCube;

            fn next(&mut self) -> Option<Self::Item> {
                let next = self.inner.next()?;
                self.rotations_checked += 1;
                Some(next)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let left = 24 - self.rotations_checked;
                (left, Some(left))
            }
        }

        impl<I> ExactSizeIterator for AllRotationsIter<I> where I: Iterator<Item = PolyCube> {}

        AllRotationsIter {
            inner: rots_in_native_plane.chain(all_others),
            rotations_checked: 0,
        }
    }
}

#[test]
pub fn correct_amount_of_rotations() {
    let cube = PolyCube::new_equal_sides(5);

    assert_eq!(cube.clone().rotations_in_plane((1, 2)).count(), 4);
    assert_eq!(cube.all_rotations().count(), 24);
}
