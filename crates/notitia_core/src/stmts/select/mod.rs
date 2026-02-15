mod join;
pub use join::*;

mod select;
pub use select::*;

mod filter;
pub use filter::*;

mod built;
pub use built::*;

/*
pub trait UnsizedExecutionResults<T> {}

impl<T> UnsizedExecutionResults<T> for Vec<T> {}
impl<T> UnsizedExecutionResults<T> for Box<[T]> {}

#[cfg(feature = "smallvec")]
impl<T, const N: usize> UnsizedExecutionResults<T> for smallvec::SmallVec<[T; N]> {}

pub trait SizedExecutionResults<T, const N: usize> {}

impl<T, const N: usize> SizedExecutionResults<T, N> for Vec<T> {}
impl<T, const N: usize> SizedExecutionResults<T, N> for Box<[T]> {}

#[cfg(feature = "smallvec")]
impl<T, const N: usize, const SN: usize> SizedExecutionResults<T, N>
    for smallvec::SmallVec<[T; SN]>
{
}

pub enum GetOneError {}
*/
