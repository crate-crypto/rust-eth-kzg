pub use rayon::{
    iter::{
        IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
    },
    join,
    slice::{ChunksMut, ParallelSliceMut},
};

pub trait MaybeParallelExt: IntoParallelIterator {
    fn maybe_into_par_iter(self) -> <Self as IntoParallelIterator>::Iter
    where
        Self: Sized,
    {
        self.into_par_iter()
    }
}

pub trait MaybeParallelRefExt: for<'a> IntoParallelRefIterator<'a> {
    fn maybe_par_iter(&self) -> <Self as IntoParallelRefIterator<'_>>::Iter {
        self.par_iter()
    }
}

pub trait MaybeParallelRefMutExt: for<'a> IntoParallelRefMutIterator<'a> {
    fn maybe_par_iter_mut(&mut self) -> <Self as IntoParallelRefMutIterator<'_>>::Iter {
        self.par_iter_mut()
    }
}

pub trait MaybeParallelSliceMut<T: Send>: ParallelSliceMut<T> {
    fn maybe_par_chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, T> {
        self.par_chunks_mut(chunk_size)
    }
}

impl<T: IntoParallelIterator> MaybeParallelExt for T {}
impl<T: for<'a> IntoParallelRefIterator<'a>> MaybeParallelRefExt for T {}
impl<T: for<'a> IntoParallelRefMutIterator<'a>> MaybeParallelRefMutExt for T {}
impl<T: Send, S: ?Sized + ParallelSliceMut<T>> MaybeParallelSliceMut<T> for S {}
