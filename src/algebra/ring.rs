use std::fmt::Debug;
use std::ops::{Add, Mul, Sub};

/// Represents an associative ring with identity
pub trait RingElement:
    Copy
    + Send
    + Sized
    + Sync
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Eq
    + Debug
{
    // multiplicative identity
    const ONE: Self;

    // additive identity
    const ZERO: Self;
}

/// Represents a module over a ring:
///
/// The elements of the module is M = R^n
/// The scalar ring is R^n
///
/// We additionally require component-wise multiplication between elements in the module.
pub trait RingModule<S: RingElement>: RingElement {
    const DIMENSION: usize;

    // action of the scalar ring upon the module:
    // s * (r_1, r_2, ..., r_dimension) = (s * r_1, s * r_2, ..., s * r_dimension)
    fn action(&self, s: S) -> Self;

    fn set(&mut self, i: usize, s: S);

    fn get(&self, i: usize) -> S;
}
