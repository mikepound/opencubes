//! A rather naive polycube implementation.

use std::collections::HashSet;

use indicatif::ProgressBar;
use parking_lot::RwLock;

use crate::pcube::RawPCube;

mod expander;
mod rotations;

/// A polycube, represented as three dimensions and an array of booleans.
///
/// The array of booleans represents the cubes and their presence (if `true`)
/// or absence (if `false`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NaivePolyCube {
    dim_1: usize,
    dim_2: usize,
    dim_3: usize,
    filled: Vec<bool>,
}

impl From<RawPCube> for NaivePolyCube {
    fn from(value: RawPCube) -> Self {
        let (d1, d2, d3) = value.dims();
        let (dim_1, dim_2, dim_3) = (d1 as usize, d2 as usize, d3 as usize);

        let mut filled = Vec::with_capacity(dim_1 * dim_2 * dim_3);

        value.data().iter().for_each(|v| {
            for s in 0..8 {
                let is_set = ((*v >> s) & 0x1) == 0x1;
                if filled.capacity() != filled.len() {
                    filled.push(is_set);
                }
            }
        });

        Self {
            dim_1,
            dim_2,
            dim_3,
            filled,
        }
    }
}

impl From<&'_ NaivePolyCube> for RawPCube {
    fn from(value: &'_ NaivePolyCube) -> Self {
        let byte_len = ((value.dim_1 * value.dim_2 * value.dim_3) + 7) / 8;

        let mut filled = value.filled.iter();

        let mut out_bytes = vec![0; byte_len];

        out_bytes.iter_mut().for_each(|v| {
            for s in 0..8 {
                if let Some(true) = filled.next() {
                    *v |= 1 << s;
                }
            }
        });

        RawPCube::new(
            value.dim_1 as u8,
            value.dim_2 as u8,
            value.dim_3 as u8,
            out_bytes,
        )
        .unwrap()
    }
}

impl From<NaivePolyCube> for RawPCube {
    fn from(value: NaivePolyCube) -> Self {
        Self::from(&value)
    }
}

/// Creating a new polycube from a triple-nested vector
/// is convenient if/when you're writing them out
/// by hand.
impl From<Vec<Vec<Vec<bool>>>> for NaivePolyCube {
    fn from(value: Vec<Vec<Vec<bool>>>) -> Self {
        let dim_1 = value.len();
        let dim_2 = value[0].len();
        let dim_3 = value[0][0].len();

        let mut poly_cube = NaivePolyCube::new(dim_1, dim_2, dim_3);

        for d3 in 0..poly_cube.dim_3 {
            for d2 in 0..poly_cube.dim_2 {
                for d1 in 0..poly_cube.dim_1 {
                    poly_cube.set_to(d1, d2, d3, value[d1][d2][d3]).unwrap();
                }
            }
        }

        poly_cube
    }
}

impl NaivePolyCube {
    /// Get the dimensions of this polycube
    pub fn dims(&self) -> (usize, usize, usize) {
        (self.dim_1, self.dim_2, self.dim_3)
    }

    pub fn present_cubes(&self) -> usize {
        self.filled.iter().filter(|v| **v).count()
    }

    /// Find the ordering between two rotated versions of the same
    /// PolyCube.
    ///
    /// This function only produces valid results if `self` and `other` are
    /// two different rotations of the same PolyCube.
    fn canonical_ordering(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;

        macro_rules! check_next {
            ($name:ident) => {
                match self.$name.cmp(&other.$name) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
            };
        }

        check_next!(dim_1);
        check_next!(dim_2);
        check_next!(dim_3);

        self.filled.cmp(&other.filled)
    }

    /// Find the canonical form of this PolyCube
    pub fn canonical_form(&self) -> Self {
        self.all_rotations()
            .max_by(Self::canonical_ordering)
            .unwrap()
    }

    /// Find the PCube-file canonical form of this PolyCube.
    pub fn pcube_canonical_form(&self) -> Self {
        self.all_rotations()
            .map(RawPCube::from)
            .max_by(RawPCube::canonical_cmp)
            .map(Into::into)
            .unwrap()
    }

    /// Calculate the offset into `self.filled` using the provided offsets
    /// within each dimension.
    fn offset(&self, dim_1: usize, dim_2: usize, dim_3: usize) -> Option<usize> {
        if dim_1 < self.dim_1 && dim_2 < self.dim_2 && dim_3 < self.dim_3 {
            let d1 = dim_1 * self.dim_2 * self.dim_3;
            let d2 = dim_2 * self.dim_3;
            let d3 = dim_3;
            let index = d1 + d2 + d3;
            Some(index)
        } else {
            None
        }
    }

    pub fn new_raw(dim_1: usize, dim_2: usize, dim_3: usize, filled: Vec<bool>) -> Self {
        Self {
            dim_1,
            dim_2,
            dim_3,
            filled,
        }
    }

    /// Create a new [`NaivePolyCube`] with dimensions `(dim_1, dim_2, dim_3)`.
    pub fn new(dim_1: usize, dim_2: usize, dim_3: usize) -> Self {
        let filled = (0..dim_1 * dim_2 * dim_3).map(|_| false).collect();

        Self {
            dim_1,
            dim_2,
            dim_3,
            filled,
        }
    }

    /// Create a new [`NaivePolyCube`] with dimensions `(side, side, side)`.
    pub fn new_equal_sides(side: usize) -> Self {
        Self::new(side, side, side)
    }

    /// Set the state of the box located at `(d1, d2, d3)` to `set`.
    pub fn set_to(&mut self, d1: usize, d2: usize, d3: usize, set: bool) -> Result<(), ()> {
        let idx = self.offset(d1, d2, d3).ok_or(())?;
        self.filled[idx] = set;
        Ok(())
    }

    /// Set the box located at `(d1, d2, d3)` to be filled.
    pub fn set(&mut self, d1: usize, d2: usize, d3: usize) -> Result<(), ()> {
        self.set_to(d1, d2, d3, true)
    }

    /// Returns whether the box located at `(d1, d2, d3)` is filled.
    pub fn is_set(&self, d1: usize, d2: usize, d3: usize) -> bool {
        self.offset(d1, d2, d3)
            .map(|v| self.filled[v])
            .unwrap_or(false)
    }

    /// Create a new [`NaivePolyCube`], representing `self` rotated `k` times in the plane indicated by `a1` and `a2`.
    pub fn rot90(mut self, k: usize, (a1, a2): (usize, usize)) -> NaivePolyCube {
        assert!(a1 <= 2, "a1 must be <= 2");
        assert!(a2 <= 2, "a2 must be <= 2");

        let k = k % 4;

        if k == 0 {
            return self;
        }

        if k == 2 {
            self.flip(a1);
            self.flip(a2);
            return self;
        }

        let mut axes: [usize; 3] = [0, 1, 2];
        let saved = axes[a1];
        axes[a1] = axes[a2];
        axes[a2] = saved;

        if k == 1 {
            self.flip(a2);
            self.transpose(axes[0], axes[1], axes[2])
        } else {
            // k == 3
            let mut transposed = self.transpose(axes[0], axes[1], axes[2]);
            transposed.flip(a2);
            transposed
        }
    }

    /// Create a new [`NaivePolyCube`], representing `self` transposed according to `a1`, `a2`, and `a3`.
    ///
    /// The axes of the returned [`NaivePolyCube`] will be those of `self`, rearranged according to the
    /// provided axes.
    pub fn transpose(&self, a1: usize, a2: usize, a3: usize) -> NaivePolyCube {
        assert!(a1 != a2);
        assert!(a1 != a3);
        assert!(a2 != a3);
        assert!(a1 <= 2);
        assert!(a2 <= 2);
        assert!(a3 <= 2);

        let original_dimension = [self.dim_1, self.dim_2, self.dim_3];
        let [td1, td2, td3] = [
            original_dimension[a1],
            original_dimension[a2],
            original_dimension[a3],
        ];

        let mut new_cube = NaivePolyCube::new(td1, td2, td3);

        for d1 in 0..self.dim_1 {
            for d2 in 0..self.dim_2 {
                for d3 in 0..self.dim_3 {
                    let original = [d1, d2, d3];
                    let [t1, t2, t3] = [original[a1], original[a2], original[a3]];

                    let orig_idx = self.offset(d1, d2, d3).unwrap();
                    let transposed_idx = new_cube.offset(t1, t2, t3).unwrap();

                    new_cube.filled[transposed_idx] = self.filled[orig_idx];
                }
            }
        }

        new_cube
    }

    /// Create a new [`NaivePolyCube`], representing `self` flipped along `axis`.
    pub fn flip(&mut self, axis: usize) {
        assert!(axis <= 2, "Axis must be <= 2");

        let d1_len = self.dim_2 * self.dim_3;
        let d2_len = self.dim_3;
        let mut cache = [false; 256];

        match axis {
            0 => {
                for from in 0..self.dim_1 / 2 {
                    let from_start = from * d1_len;
                    let from_end = (from + 1) * d1_len;

                    let to_start = (self.dim_1 - from - 1) * d1_len;
                    let to_end = (self.dim_1 - from) * d1_len;

                    cache[..d1_len].copy_from_slice(&self.filled[from_start..from_end]);
                    self.filled.copy_within(to_start..to_end, from_start);
                    self.filled[to_start..to_end].copy_from_slice(&cache[0..d1_len]);
                }
            }
            1 => {
                let d1_range = 0..self.dim_1;

                for d1 in d1_range.map(|v| v * d1_len) {
                    for from in 0..self.dim_2 / 2 {
                        let from_start = d1 + from * d2_len;
                        let from_end = d1 + (from + 1) * d2_len;

                        let to_start = d1 + (self.dim_2 - from - 1) * d2_len;
                        let to_end = d1 + (self.dim_2 - from) * d2_len;

                        cache[0..d2_len].copy_from_slice(&self.filled[from_start..from_end]);
                        self.filled.copy_within(to_start..to_end, from_start);
                        self.filled[to_start..to_end].copy_from_slice(&cache[..d2_len]);
                    }
                }
            }
            2 => {
                let d1_range = 0..self.dim_1;
                let d2_range = 0..self.dim_2;

                for d1 in d1_range.map(|v| v * d1_len) {
                    for d2 in d2_range.clone().map(|v| v * d2_len) {
                        for d3 in 0..self.dim_3 / 2 {
                            let from = d1 + d2 + d3;
                            let to = (d1 + d2 + self.dim_3) - d3 - 1;

                            self.filled.swap(from, to);
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    /// Create a new [`NaivePolyCube`] that has an extra box-space on all sides
    /// of the polycube.
    pub fn pad_one(&self) -> NaivePolyCube {
        let mut cube_next = NaivePolyCube::new(self.dim_1 + 2, self.dim_2 + 2, self.dim_3 + 2);

        for d1 in 0..self.dim_1 {
            for d2 in 0..self.dim_2 {
                for d3 in 0..self.dim_3 {
                    cube_next
                        .set_to(d1 + 1, d2 + 1, d3 + 1, self.is_set(d1, d2, d3))
                        .unwrap();
                }
            }
        }

        cube_next
    }

    /// Obtain a list of [`NaivePolyCube`]s representing all unique expansions of the
    /// items in `from_set`.
    ///
    // TODO: turn this into an iterator that yield unique expansions?
    pub fn unique_expansions<'a, I>(progress_bar: &ProgressBar, from_set: I) -> Vec<NaivePolyCube>
    where
        I: Iterator<Item = &'a NaivePolyCube> + ExactSizeIterator,
    {
        let mut this_level = HashSet::new();

        for value in from_set {
            for expansion in value.expand().map(|v| v.crop()) {
                // Skip expansions that are already in the list.
                if this_level.contains(&expansion) {
                    continue;
                }

                let max = expansion.canonical_form();

                let missing = !this_level.contains(&max);

                if missing {
                    this_level.insert(max);
                }
            }

            progress_bar.inc(1);
        }

        this_level.into_iter().collect()
    }

    /// Check whether this cube is already cropped.
    pub fn is_cropped(&self) -> bool {
        macro_rules! direction {
            ($d1:expr, $d2:expr, $d3:expr, $pred:expr) => {{
                for d1 in $d1 {
                    let mut has_nonzero = false;
                    for d2 in 0..$d2 {
                        for d3 in 0..$d3 {
                            has_nonzero |= $pred(d1, d2, d3);
                            if has_nonzero {
                                break;
                            }
                        }
                    }

                    if !has_nonzero {
                        return false;
                    } else {
                        break;
                    }
                }
            }};
        }

        let d1_first = |d1, d2, d3| self.is_set(d1, d2, d3);
        direction!(0..self.dim_1, self.dim_2, self.dim_3, d1_first);
        direction!((0..self.dim_1).rev(), self.dim_2, self.dim_3, d1_first);

        let d2_first = |d2, d1, d3| self.is_set(d1, d2, d3);
        direction!(0..self.dim_2, self.dim_1, self.dim_3, d2_first);
        direction!((0..self.dim_2).rev(), self.dim_1, self.dim_3, d2_first);

        let d3_first = |d3, d1, d2| self.is_set(d1, d2, d3);
        direction!(0..self.dim_3, self.dim_1, self.dim_2, d3_first);
        direction!((0..self.dim_3).rev(), self.dim_1, self.dim_2, d3_first);

        return true;
    }

    /// Create a new [`NaivePolyCube`] representing `self` but cropped.
    ///
    /// Cropping means that there are no planes without any present boxes.
    pub fn crop(&self) -> NaivePolyCube {
        macro_rules! direction {
            ($d1:expr, $d2:expr, $d3:expr, $pred:expr) => {{
                let mut all_zero_count: usize = 0;

                for d1 in $d1 {
                    let mut has_nonzero = false;
                    for d2 in 0..$d2 {
                        for d3 in 0..$d3 {
                            has_nonzero |= $pred(d1, d2, d3);
                            if has_nonzero {
                                break;
                            }
                        }
                    }

                    if !has_nonzero {
                        all_zero_count += 1;
                    } else {
                        break;
                    }
                }

                all_zero_count
            }};
        }

        let d1_first = |d1, d2, d3| self.is_set(d1, d2, d3);
        let d1_left = direction!(0..self.dim_1, self.dim_2, self.dim_3, d1_first);

        // If there are `dim_1` planes to be removed, we have to remove them all,
        // which means that there are no boxes present in this polycube, at all.
        if d1_left == self.dim_1 {
            return NaivePolyCube {
                dim_1: 0,
                dim_2: 0,
                dim_3: 0,
                filled: Vec::new(),
            };
        }

        let d1_right = direction!((0..self.dim_1).rev(), self.dim_2, self.dim_3, d1_first);

        let d2_first = |d2, d1, d3| self.is_set(d1, d2, d3);
        let d2_left = direction!(0..self.dim_2, self.dim_1, self.dim_3, d2_first);
        let d2_right = direction!((0..self.dim_2).rev(), self.dim_1, self.dim_3, d2_first);

        let d3_first = |d3, d1, d2| self.is_set(d1, d2, d3);
        let d3_left = direction!(0..self.dim_3, self.dim_1, self.dim_2, d3_first);
        let d3_right = direction!((0..self.dim_3).rev(), self.dim_1, self.dim_2, d3_first);

        let mut new_cube = NaivePolyCube::new(
            self.dim_1 - d1_left - d1_right,
            self.dim_2 - d2_left - d2_right,
            self.dim_3 - d3_left - d3_right,
        );

        for d1 in 0..new_cube.dim_1 {
            for d2 in 0..new_cube.dim_2 {
                for d3 in 0..new_cube.dim_3 {
                    let d1_from = d1 + d1_left;
                    let d2_from = d2 + d2_left;
                    let d3_from = d3 + d3_left;

                    let is_set = self.is_set(d1_from, d2_from, d3_from);
                    new_cube.set_to(d1, d2, d3, is_set).unwrap();
                }
            }
        }

        new_cube
    }
}

impl NaivePolyCube {
    // TODO: turn this into an iterator that yield unique expansions?
    pub fn unique_expansions_rayon<'a, I>(bar: &ProgressBar, from_set: I) -> Vec<NaivePolyCube>
    where
        I: Iterator<Item = &'a NaivePolyCube> + ExactSizeIterator + Clone + Send + Sync,
    {
        use rayon::prelude::*;

        if from_set.len() == 0 {
            return Vec::new();
        }

        let available_parallelism = num_cpus::get();

        let chunk_size = (from_set.len() / available_parallelism) + 1;
        let chunks = (from_set.len() + chunk_size - 1) / chunk_size;

        let chunk_iterator = (0..chunks).into_par_iter().map(|v| {
            from_set
                .clone()
                .skip(v * chunk_size)
                .take(chunk_size)
                .into_iter()
        });

        let this_level = RwLock::new(HashSet::new());

        chunk_iterator.for_each(|v| {
            for value in v {
                for expansion in value.expand().map(|v| v.crop()) {
                    // Skip expansions that are already in the list.
                    if this_level.read().contains(&expansion) {
                        continue;
                    }

                    let max = expansion.canonical_form();

                    let missing = !this_level.read().contains(&max);

                    if missing {
                        this_level.write().insert(max);
                    }
                }

                bar.inc(1);
            }
        });

        this_level.into_inner().into_iter().collect()
    }
}
