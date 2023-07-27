use std::collections::{HashMap, HashSet};

use crate::pcube::RawPCube;

/// An iterator over polycubes
pub trait PolycubeIterator: Iterator<Item = RawPCube>
where
    Self: Sized,
{
    /// Returns true if all polycubes returned are in _some_ canonical
    /// form. No guarantee is provided about the type of canonicality, nor
    /// about uniqueness. However, if this returns `true` it is guaranteed
    /// that all cubes returned by this iterator are in a form that can be
    /// used directly to check for uniqueness.
    fn is_canonical(&self) -> bool;

    fn n_hint(&self) -> Option<usize>;
}

/// A trait for converting a [`PolycubeIterator`] into a [`UniquePolycubeIterator`].
pub trait IntoUniquePolycubeIterator
where
    Self: Sized + PolycubeIterator,
{
    fn into_unique(self) -> UniquePolycubes<Self> {
        UniquePolycubes::new(self)
    }
}

impl<T> IntoUniquePolycubeIterator for T where T: PolycubeIterator {}

/// An iterator over at least one variant of all unique polycubes
/// of size [`n`](AllPolycubeIterator::n).
///
/// Iterators that implement this trait guarantee that they yield
/// at least one copy of all polycubes for size `n`, but do not guarantee
/// anything about the orientation of those cubes, nor about the amount
/// of times each copy of that polycubes occurs.
pub trait AllPolycubeIterator: PolycubeIterator {
    /// The size of the polycubes returned by this
    /// iterator.
    fn n(&self) -> usize {
        let n_hint = self.n_hint();
        assert!(n_hint.is_some());
        // SAFETY: we asserted that n_hint is some
        unsafe { n_hint.unwrap_unchecked() }
    }
}

/// An iterator over unique polycubes.
///
/// Unique, in this context, means that no two items yielded by this
/// iterator have the same canonical form.
pub trait UniquePolycubeIterator: PolycubeIterator {}

/// An iterator over all unique polycubes of size [`n`](AllPolycubeIterator::n).
pub trait AllUniquePolycubeIterator: UniquePolycubeIterator + AllPolycubeIterator {}

/// A struct that yields only unique Polycubes.
pub struct UniquePolycubes<T> {
    stored: HashMap<(u8, u8, u8), HashSet<Vec<u8>>>,
    inner: T,
}

impl<T> UniquePolycubes<T>
where
    T: PolycubeIterator,
{
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            stored: HashMap::new(),
        }
    }
}

impl<T> Iterator for UniquePolycubes<T>
where
    T: Iterator<Item = RawPCube>,
{
    type Item = RawPCube;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(v) = self.inner.next() {
            let entry = self.stored.entry(v.dims()).or_default();

            // No need to canonicalize, as a `UniquePolycubes` can only be constructed
            // from a `PolycubeIterator` that is canonical.

            if entry.contains(v.data()) {
                continue;
            }

            if entry.insert(v.data().to_vec()) {
                return Some(v);
            }
        }

        None
    }
}

impl<T> PolycubeIterator for UniquePolycubes<T>
where
    T: PolycubeIterator<Item = RawPCube>,
{
    fn n_hint(&self) -> Option<usize> {
        self.inner.n_hint()
    }

    fn is_canonical(&self) -> bool {
        let is_canonical = self.inner.is_canonical();
        assert!(is_canonical);
        is_canonical
    }
}

impl<T> UniquePolycubeIterator for UniquePolycubes<T> where T: PolycubeIterator<Item = RawPCube> {}

impl<T> AllPolycubeIterator for UniquePolycubes<T>
where
    T: AllPolycubeIterator,
{
    fn n(&self) -> usize {
        self.inner.n()
    }
}

impl<T> AllUniquePolycubeIterator for UniquePolycubes<T> where T: AllPolycubeIterator {}

// TODO: hide this behind a feature?
pub mod indicatif {
    use indicatif::ProgressBar;

    use super::{
        AllPolycubeIterator, AllUniquePolycubeIterator, PolycubeIterator, UniquePolycubeIterator,
    };

    pub struct PolycubeProgressBarIter<T> {
        bar: ProgressBar,
        inner: T,
    }

    impl<T> PolycubeProgressBarIter<T>
    where
        T: PolycubeIterator,
    {
        pub fn new(bar: ProgressBar, inner: T) -> Self {
            Self { inner, bar }
        }
    }

    impl<T> Iterator for PolycubeProgressBarIter<T>
    where
        T: Iterator,
    {
        type Item = T::Item;

        fn next(&mut self) -> Option<Self::Item> {
            self.bar.inc(1);
            self.inner.next()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.inner.size_hint()
        }
    }

    impl<T> PolycubeIterator for PolycubeProgressBarIter<T>
    where
        T: PolycubeIterator,
    {
        fn is_canonical(&self) -> bool {
            self.inner.is_canonical()
        }

        fn n_hint(&self) -> Option<usize> {
            self.inner.n_hint()
        }
    }

    impl<T> ExactSizeIterator for PolycubeProgressBarIter<T> where T: ExactSizeIterator {}
    impl<T> AllPolycubeIterator for PolycubeProgressBarIter<T> where T: AllPolycubeIterator {}
    impl<T> UniquePolycubeIterator for PolycubeProgressBarIter<T> where T: UniquePolycubeIterator {}
    impl<T> AllUniquePolycubeIterator for PolycubeProgressBarIter<T> where T: AllUniquePolycubeIterator {}
}
