use bls12_381::{ff::Field, Scalar};

/// CosetFFt contains a generator(coset) element that can be used
/// to compute a coset FFT and its inverse which consequently can be used to
/// compute a coset IFFT
#[derive(Debug, Clone)]
pub struct CosetFFT {
    pub generator: Scalar,
    pub generator_inv: Scalar,
}

impl CosetFFT {
    pub fn new(gen: Scalar) -> Self {
        Self {
            generator: gen,
            generator_inv: gen.invert().expect("cosets should be non-zero"),
        }
    }
}
