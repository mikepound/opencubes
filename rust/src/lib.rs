#[cfg(test)]
mod test;

use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use indicatif::ProgressStyle;

mod iterator;

/// A polycube
#[derive(Debug)]
pub struct PolyCube {
    alloc_count: Rc<AtomicUsize>,
    dim_1: usize,
    dim_2: usize,
    dim_3: usize,
    dim_2_scalar: usize,
    dim_3_scalar: usize,
    filled: Vec<bool>,
}

impl Clone for PolyCube {
    fn clone(&self) -> Self {
        self.increase_alloc_count();
        Self {
            alloc_count: self.alloc_count.clone(),
            dim_1: self.dim_1.clone(),
            dim_2: self.dim_2.clone(),
            dim_3: self.dim_3.clone(),
            dim_2_scalar: self.dim_2_scalar.clone(),
            dim_3_scalar: self.dim_3_scalar.clone(),
            filled: self.filled.clone(),
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
        self.dim_1.hash(state);
        self.dim_2.hash(state);
        self.dim_3.hash(state);
        self.filled.hash(state);
    }
}

impl core::fmt::Display for PolyCube {
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
    pub fn dims(&self) -> (usize, usize, usize) {
        (self.dim_1, self.dim_2, self.dim_3)
    }

    pub fn cube_iter(&self) -> impl Iterator<Item = &bool> + '_ {
        self.filled.iter()
    }

    fn index(&self, dim_1: usize, dim_2: usize, dim_3: usize) -> Option<usize> {
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

    fn increase_alloc_count(&self) {
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn new_with_alloc_count(
        alloc_count: Rc<AtomicUsize>,
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

    pub fn alloc_count(&self) -> Rc<AtomicUsize> {
        self.alloc_count.clone()
    }

    pub fn new(dim_1: usize, dim_2: usize, dim_3: usize) -> Self {
        let filled = (0..dim_1 * dim_2 * dim_3).map(|_| false).collect();

        Self {
            alloc_count: Rc::new(AtomicUsize::new(0)),
            dim_1,
            dim_2,
            dim_3,
            dim_2_scalar: dim_1,
            dim_3_scalar: dim_1 * dim_2,
            filled,
        }
    }

    pub fn new_equal_sides(side: usize) -> Self {
        Self::new(side, side, side)
    }

    pub fn size(&self) -> usize {
        self.dim_1 * self.dim_2 * self.dim_3
    }

    pub fn set_to(&mut self, d1: usize, d2: usize, d3: usize, set: bool) -> Result<(), ()> {
        let idx = self.index(d1, d2, d3).ok_or(())?;
        self.filled[idx] = set;
        Ok(())
    }

    pub fn set(&mut self, d1: usize, d2: usize, d3: usize) -> Result<(), ()> {
        self.set_to(d1, d2, d3, true)
    }

    pub fn is_set(&self, d1: usize, d2: usize, d3: usize) -> bool {
        self.index(d1, d2, d3)
            .map(|v| self.filled[v])
            .unwrap_or(false)
    }

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

                    let orig_idx = self.index(d1, d2, d3).unwrap();
                    let transposed_idx = new_cube.index(t1, t2, t3).unwrap();

                    new_cube.filled[transposed_idx] = self.filled[orig_idx];
                }
            }
        }

        new_cube
    }

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
                            let idx_1 = self.index(d1, d2, d3).unwrap();
                            let idx_2 = $flipped_idx(d1, d2, d3).unwrap();

                            new_cube.filled[idx_2] = self.filled[idx_1];
                        }
                    }
                }
            };
        }

        match axis {
            0 => flip!(|d1, d2, d3| self.index(self.dim_1 - d1 - 1, d2, d3)),
            1 => flip!(|d1, d2, d3| self.index(d1, self.dim_2 - d2 - 1, d3)),
            2 => flip!(|d1, d2, d3| self.index(d1, d2, self.dim_3 - d3 - 1)),
            _ => unreachable!(),
        }

        new_cube
    }

    fn pad_one(&self) -> PolyCube {
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

    pub fn unique_expansions<'a, I>(from_set: I) -> Vec<PolyCube>
    where
        I: Iterator<Item = &'a PolyCube> + ExactSizeIterator,
    {
        use std::collections::HashSet;

        #[cfg(feature = "indicatif")]
        let bar = {
            let bar = indicatif::ProgressBar::new(from_set.len() as u64);

            let pos_width = format!("{}", from_set.len()).len();

            let template = format!(
                "{{spinner:.green}} {{bar:40.cyan/blue}} {{pos:>{pos_width}}}/{{len}} {{msg}}"
            );

            bar.set_style(
                ProgressStyle::with_template(&template)
                    .unwrap()
                    .progress_chars("#>-"),
            );
            bar
        };

        let mut this_level = HashSet::new();

        for value in from_set {
            for expansion in value.expand().map(|v| v.crop()) {
                let missing = !expansion.all_rotations().any(|v| this_level.contains(&v));

                if missing {
                    this_level.insert(expansion);
                }
            }

            #[cfg(feature = "indicatif")]
            bar.inc(1);
        }

        #[cfg(feature = "indicatif")]
        bar.finish();

        this_level.into_iter().collect()
    }

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

        if d1_left == self.dim_1 {
            return PolyCube {
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
