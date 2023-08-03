// hopefully only expose pcube reps in the future not full modules
pub mod point_list;

pub mod naive_polycube;

pub mod pcube;
pub use self::pcube::RawPCube;

pub mod rotation_reduced;

/// the "Dimension" or "Shape" of a poly cube
/// defines the maximum bounds of a polycube
/// X >= Y >= Z for efficiency reasons reducing the number of rotations needing to be performed
/// stores len() for each dimension so the unit cube has a size of (1, 1, 1)
/// and the 2x1x1 starting seed has a dimension of (2, 1, 1)
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct Dim {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

pub trait PolyCube: From<RawPCube> + Into<RawPCube> + Sized {
    /// Produce an iterator that yields all unique n + 1 expansions of
    /// `input`.
    fn unique_expansions<'a>(&'a self) -> Box<dyn Iterator<Item = Self> + 'a>;

    /// Return a copy of self in some "canonical" form.
    fn canonical_form(&self) -> Self;
    fn size(&self) -> usize;
    fn dims(&self) -> Dim;
}
