#[cfg(test)]
mod test;

use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

mod iterator;

/// A polycube
#[derive(Debug)]
pub struct PolyCube {
    alloc_count: Arc<AtomicUsize>,
    dim_1: usize,
    dim_2: usize,
    dim_3: usize,
    dim_2_scalar: usize,
    dim_3_scalar: usize,
    filled: Vec<bool>,
}

impl Clone for PolyCube {
    fn clone(&self) -> Self {
        // If `filled` is empty, cloning the vector is unnecessary.
        // We can avoid an allocation by just creating a `new` Vec instead.
        let filled = if !self.filled.is_empty() {
            self.increase_alloc_count();
            self.filled.clone()
        } else {
            Vec::new()
        };

        Self {
            alloc_count: self.alloc_count.clone(),
            dim_1: self.dim_1.clone(),
            dim_2: self.dim_2.clone(),
            dim_3: self.dim_3.clone(),
            dim_2_scalar: self.dim_2_scalar.clone(),
            dim_3_scalar: self.dim_3_scalar.clone(),
            filled,
        }
    }
}

impl Eq for PolyCube {}

impl PartialEq for PolyCube {
    fn eq(&self, other: &Self) -> bool {
        self.dim_1 == other.dim_1
            && self.dim_2 == other.dim_2
            && self.dim_3 == other.dim_3
            && self.filled == other.filled
    }
}

impl std::hash::Hash for PolyCube {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // For hashing purposes we do not care about the allocation tracker,
        // as that is only interesting metadata to look at and it does not
        // describe the actual state of the PolyCube.
        self.dim_1.hash(state);
        self.dim_2.hash(state);
        self.dim_3.hash(state);
        self.filled.hash(state);
    }
}

impl core::fmt::Display for PolyCube {
    // Format the polycube in a somewhat more easy to digest
    // format.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut xy = String::new();

        for _ in 0..self.dim_3 {
            xy.push('-');
        }
        xy.push('\n');

        for x in 0..self.dim_1 {
            for y in 0..self.dim_2 {
                for z in 0..self.dim_3 {
                    if self.is_set(x, y, z) {
                        xy.push('1');
                    } else {
                        xy.push('0');
                    }
                }
                xy.push('\n');
            }

            for _ in 0..self.dim_3 {
                xy.push('-');
            }
            xy.push('\n');
        }

        write!(f, "{}", xy.trim_end())
    }
}

/// Creating a new polycube from a triple-nested vector
/// is convenient if/when you're writing them out
/// by hand.
impl From<Vec<Vec<Vec<bool>>>> for PolyCube {
    fn from(value: Vec<Vec<Vec<bool>>>) -> Self {
        let dim_1 = value.len();
        let dim_2 = value[0].len();
        let dim_3 = value[0][0].len();

        let mut poly_cube = PolyCube::new(dim_1, dim_2, dim_3);

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

impl PolyCube {
    /// Get the dimensions of this polycube
    pub fn dims(&self) -> (usize, usize, usize) {
        (self.dim_1, self.dim_2, self.dim_3)
    }

    /// Find the ordering between two rotated versions of the same
    /// PolyCube.
    ///
    /// This function only produces valid results if `self` and `other` are
    /// two different rotations of the same PolyCube.
    pub fn canonical_ordering(&self, other: &Self) -> core::cmp::Ordering {
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
        // I don't think this does what I expect it to do...
        self.filled.cmp(&other.filled)
    }

    /// Calculate the offset into `self.filled` using the provided offsets
    /// within each dimension.
    fn offset(&self, dim_1: usize, dim_2: usize, dim_3: usize) -> Option<usize> {
        if dim_1 < self.dim_1 && dim_2 < self.dim_2 && dim_3 < self.dim_3 {
            let d1 = dim_1;
            let d2 = dim_2 * self.dim_2_scalar;
            let d3 = dim_3 * self.dim_3_scalar;
            let index = d1 + d2 + d3;
            Some(index)
        } else {
            None
        }
    }

    /// Inrease the allocation count
    fn increase_alloc_count(&self) {
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Create a new [`PolyCube`] with dimensions `(dim_1, dim_2, dim_3)`, and
    /// using `alloc_count` to keep track of the amount of [`PolyCube`]s that
    /// are allocated.
    pub fn new_with_alloc_count(
        alloc_count: Arc<AtomicUsize>,
        dim_1: usize,
        dim_2: usize,
        dim_3: usize,
    ) -> Self {
        let filled = (0..dim_1 * dim_2 * dim_3).map(|_| false).collect();

        let me = Self {
            alloc_count,
            dim_1,
            dim_2,
            dim_3,
            dim_2_scalar: dim_1,
            dim_3_scalar: dim_1 * dim_2,
            filled,
        };

        me.increase_alloc_count();

        me
    }

    /// Get the amount of allocations that have been
    /// performed by this [`PolyCube`]
    pub fn alloc_count(&self) -> usize {
        self.alloc_count.load(Ordering::Relaxed)
    }

    /// Create a new [`PolyCube`] with dimensions `(dim_1, dim_2, dim_3)` and
    /// a new allocation tracker.
    pub fn new(dim_1: usize, dim_2: usize, dim_3: usize) -> Self {
        let filled = (0..dim_1 * dim_2 * dim_3).map(|_| false).collect();

        Self {
            alloc_count: Arc::new(AtomicUsize::new(0)),
            dim_1,
            dim_2,
            dim_3,
            dim_2_scalar: dim_1,
            dim_3_scalar: dim_1 * dim_2,
            filled,
        }
    }

    /// Create a new [`PolyCube`] with dimensions `(side, side, side)`, and
    /// a new allocation tracker.
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

    /// Create a new [`PolyCube`], representing `self` rotated `k` times in the plane indicated by `a1` and `a2`.
    pub fn rot90(self, k: usize, (a1, a2): (usize, usize)) -> PolyCube {
        assert!(a1 <= 2, "a1 must be <= 2");
        assert!(a2 <= 2, "a2 must be <= 2");

        let k = k % 4;

        if k == 0 {
            return self;
        }

        if k == 2 {
            return self.flip(a1).flip(a2);
        }

        let mut axes: [usize; 3] = [0, 1, 2];
        let saved = axes[a1];
        axes[a1] = axes[a2];
        axes[a2] = saved;

        if k == 1 {
            self.flip(a2).transpose(axes[0], axes[1], axes[2])
        } else {
            // k == 3
            self.transpose(axes[0], axes[1], axes[2]).flip(a2)
        }
    }

    /// Create a new [`PolyCube`], representing `self` transposed according to `a1`, `a2`, and `a3`.
    ///
    /// The axes of the returned [`PolyCube`] will be those of `self`, rearranged according to the
    /// provided axes.
    pub fn transpose(&self, a1: usize, a2: usize, a3: usize) -> PolyCube {
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

        let mut new_cube = PolyCube::new(td1, td2, td3);

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

    /// Create a new [`PolyCube`], representing `self` flipped along `axis`.
    pub fn flip(&self, axis: usize) -> PolyCube {
        assert!(axis <= 2, "Axis must be <= 2");

        let mut new_cube = PolyCube::new_with_alloc_count(
            self.alloc_count.clone(),
            self.dim_1,
            self.dim_2,
            self.dim_3,
        );

        macro_rules! flip {
            ($flipped_idx:expr) => {
                for d1 in 0..self.dim_1 {
                    for d2 in 0..self.dim_2 {
                        for d3 in 0..self.dim_3 {
                            let idx_1 = self.offset(d1, d2, d3).unwrap();
                            let idx_2 = $flipped_idx(d1, d2, d3).unwrap();

                            new_cube.filled[idx_2] = self.filled[idx_1];
                        }
                    }
                }
            };
        }

        match axis {
            0 => flip!(|d1, d2, d3| self.offset(self.dim_1 - d1 - 1, d2, d3)),
            1 => flip!(|d1, d2, d3| self.offset(d1, self.dim_2 - d2 - 1, d3)),
            2 => flip!(|d1, d2, d3| self.offset(d1, d2, self.dim_3 - d3 - 1)),
            _ => unreachable!(),
        }

        new_cube
    }

    /// Create a new [`PolyCube`] that has an extra box-space on all sides
    /// of the polycube.
    pub fn pad_one(&self) -> PolyCube {
        let mut cube_next = PolyCube::new_with_alloc_count(
            self.alloc_count.clone(),
            self.dim_1 + 2,
            self.dim_2 + 2,
            self.dim_3 + 2,
        );

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

    #[cfg(feature = "indicatif")]
    fn make_bar(len: usize) -> indicatif::ProgressBar {
        use indicatif::{ProgressBar, ProgressStyle};

        let bar = ProgressBar::new(len as u64);

        let pos_width = format!("{len}").len();

        let template = format!(
            "[{{elapsed_precise}}] {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} {{msg}}"
        );

        bar.set_style(
            ProgressStyle::with_template(&template)
                .unwrap()
                .progress_chars("#>-"),
        );
        bar
    }

    /// Obtain a list of [`PolyCube`]s representing all unique expansions of the
    /// items in `from_set`.
    ///
    /// If the feature `indicatif` is enabled, this also prints a progress bar.
    pub fn unique_expansions<'a, I>(_n: usize, from_set: I) -> Vec<PolyCube>
    where
        I: Iterator<Item = &'a PolyCube> + ExactSizeIterator,
    {
        #[cfg(feature = "indicatif")]
        let bar = Self::make_bar(from_set.len());

        let mut this_level = HashSet::new();

        let mut iter = 0;
        for value in from_set {
            iter += 1;
            for expansion in value.expand().map(|v| v.crop()) {
                let missing = !expansion.all_rotations().any(|v| this_level.contains(&v));

                if missing {
                    this_level.insert(expansion);
                }
            }

            #[cfg(feature = "indicatif")]
            {
                bar.inc(1);

                // Try to avoid doing this too often
                if iter % (this_level.len() / 100).max(100) == 0 {
                    bar.set_message(format!(
                        "Unique polycubes for N = {} so far: {}",
                        _n,
                        this_level.len()
                    ));
                }
            }
        }

        #[cfg(feature = "indicatif")]
        {
            bar.set_message(format!(
                "Unique polycubes for N = {}: {}",
                _n,
                this_level.len(),
            ));
            bar.tick();
            bar.finish();
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

    /// Create a new [`PolyCube`] representing `self` but cropped.
    ///
    /// Cropping means that there are no planes without any present boxes.
    pub fn crop(&self) -> PolyCube {
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
            return PolyCube {
                // NOTE: this doesn't increase allocation count, since
                // Vec::new() does not allocate for size 0.
                alloc_count: self.alloc_count.clone(),
                dim_1: 0,
                dim_2: 0,
                dim_3: 0,
                dim_2_scalar: 0,
                dim_3_scalar: 0,
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

        let mut new_cube = PolyCube::new_with_alloc_count(
            self.alloc_count.clone(),
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
