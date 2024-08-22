pub use rayon::iter::IntoParallelIterator;
pub use rayon::iter::IntoParallelRefIterator;
pub use rayon::iter::IntoParallelRefMutIterator;
pub use rayon::iter::ParallelIterator;

pub trait MaybeParallelExt: IntoParallelIterator {
    fn maybe_into_par_iter(self) -> <Self as IntoParallelIterator>::Iter
    where
        Self: Sized,
    {
        self.into_par_iter()
    }
}

pub trait MaybeParallelRefExt: for<'a> IntoParallelRefIterator<'a> {
    fn maybe_par_iter(&self) -> <Self as IntoParallelRefIterator>::Iter {
        self.par_iter()
    }
}

pub trait MaybeParallelRefMutExt: for<'a> IntoParallelRefMutIterator<'a> {
    fn maybe_par_iter_mut(&mut self) -> <Self as IntoParallelRefMutIterator>::Iter {
        self.par_iter_mut()
    }
}

impl<T: IntoParallelIterator> MaybeParallelExt for T {}
impl<T: for<'a> IntoParallelRefIterator<'a>> MaybeParallelRefExt for T {}
impl<T: for<'a> IntoParallelRefMutIterator<'a>> MaybeParallelRefMutExt for T {}
