pub use std::{
    iter::{IntoIterator, Iterator},
    slice::ChunksMut,
};

#[inline]
pub fn join<A, B, RA, RB>(oper_a: A, oper_b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    (oper_a(), oper_b())
}

pub trait MaybeParallelExt: IntoIterator {
    fn maybe_into_par_iter(self) -> <Self as IntoIterator>::IntoIter
    where
        Self: Sized,
    {
        self.into_iter()
    }
}

pub trait MaybeParallelRefExt {
    type Item;
    type Iter<'a>: Iterator<Item = &'a Self::Item>
    where
        Self: 'a;
    fn maybe_par_iter(&self) -> Self::Iter<'_>;
}

pub trait MaybeParallelRefMutExt {
    type Item;
    type Iter<'a>: Iterator<Item = &'a mut Self::Item>
    where
        Self: 'a;
    fn maybe_par_iter_mut(&mut self) -> Self::Iter<'_>;
}

pub trait MaybeParallelSliceMut<T> {
    fn maybe_par_chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, T>;
}

impl<T: IntoIterator> MaybeParallelExt for T {}

impl<T: IntoIterator> MaybeParallelRefExt for T
where
    for<'a> &'a T: IntoIterator<Item = &'a <T as IntoIterator>::Item>,
{
    type Item = <T as IntoIterator>::Item;
    type Iter<'a>
        = <&'a T as IntoIterator>::IntoIter
    where
        Self: 'a;

    fn maybe_par_iter(&self) -> Self::Iter<'_> {
        self.into_iter()
    }
}

impl<T: IntoIterator> MaybeParallelRefMutExt for T
where
    for<'a> &'a mut T: IntoIterator<Item = &'a mut <T as IntoIterator>::Item>,
{
    type Item = <T as IntoIterator>::Item;
    type Iter<'a>
        = <&'a mut T as IntoIterator>::IntoIter
    where
        Self: 'a;

    fn maybe_par_iter_mut(&mut self) -> Self::Iter<'_> {
        self.into_iter()
    }
}

impl<T: Send> MaybeParallelSliceMut<T> for [T] {
    fn maybe_par_chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, T> {
        self.chunks_mut(chunk_size)
    }
}
