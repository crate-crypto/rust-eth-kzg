use bls12_381::{traits::*, Scalar};

/// Represents a coset FFT configuration over a finite field.
///
/// This struct stores a coset generator and its inverse,
/// allowing efficient computation of FFTs and IFFTs over multiplicative cosets.
#[derive(Debug, Clone)]
pub struct CosetFFT {
    /// The coset generator element `g`, used to shift the evaluation domain.
    ///
    /// The FFT is computed over the domain `g * H`, where `H` is a multiplicative subgroup.
    pub generator: Scalar,

    /// The multiplicative inverse of the coset generator `g⁻¹`.
    ///
    /// Used for computing inverse FFTs over the shifted domain.
    pub generator_inv: Scalar,
}

impl CosetFFT {
    /// Creates a new `CosetFFT` instance from a given non-zero generator.
    ///
    /// # Panics
    /// Panics if the generator is zero (i.e., has no multiplicative inverse).
    pub fn new(generator: Scalar) -> Self {
        Self {
            generator,
            generator_inv: generator.invert().expect("cosets should be non-zero"),
        }
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::Scalar;

    use super::*;

    #[test]
    #[should_panic]
    fn test_coset_fft_new_panics_on_zero_generator() {
        let zero = Scalar::ZERO;
        // This should panic because zero has no multiplicative inverse
        let _ = CosetFFT::new(zero);
    }
}
