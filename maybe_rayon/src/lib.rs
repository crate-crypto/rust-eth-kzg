#[cfg(feature = "multithreading")]
mod multi_threaded;
#[cfg(not(feature = "multithreading"))]
mod single_threaded;

#[cfg(feature = "multithreading")]
pub use multi_threaded::*;
#[cfg(not(feature = "multithreading"))]
pub use single_threaded::*;

pub mod prelude {
    pub use crate::MaybeParallelRefExt;
    pub use crate::MaybeParallelRefMutExt;
    pub use crate::*;
    #[cfg(feature = "multithreading")]
    pub use rayon::prelude::*;
}
