#[cfg(feature = "multithreaded")]
mod multi_threaded;
#[cfg(not(feature = "multithreaded"))]
mod single_threaded;

#[cfg(feature = "multithreaded")]
pub use multi_threaded::*;
#[cfg(not(feature = "multithreaded"))]
pub use single_threaded::*;

pub mod prelude {
    #[cfg(feature = "multithreaded")]
    pub use rayon::prelude::*;

    pub use crate::{MaybeParallelRefExt, MaybeParallelRefMutExt, *};
}
