mod array;
mod reader;
mod subset;
mod vec;
mod writer;

pub use array::*;
pub use reader::*;
pub use subset::*;
pub use vec::VecMap;
pub use writer::*;

use std::mem;

pub const fn log2(x: usize) -> usize {
    (mem::size_of::<usize>() * 8) - (x.leading_zeros() as usize) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log2() {
        assert_eq!(log2(1024), 10);
    }
}