use blst::blst_p1_affine;
use blstrs::{Fp, G1Affine};
use ff::PrimeField;

use crate::{
    fixed_base_msm_window::FixedBaseMSMPrecompWindow, lincomb::g1_lincomb, G1Projective, Scalar,
};

/// A precomputed structure for performing fixed-base multi-scalar multiplication (MSM) in G1 using BLST.
///
/// This structure is optimized for the case where the base points (generators) are fixed across multiple
/// MSM invocations. It uses the windowed method with precomputed lookup tables to reduce computation time.
///
/// The precomputation amortizes the cost of scalar multiplication by building a table of multiples
/// for each generator point up to a certain window size.
///
/// The memory and performance trade-off is configurable via the `wbits` parameter:
/// - Larger `wbits` → faster runtime, higher memory.
/// - Smaller `wbits` → lower memory, slower MSM.
#[derive(Debug)]
pub struct FixedBaseMSMPrecompBLST {
    /// Precomputed lookup table containing multiples of all fixed G1 generator points.
    ///
    /// The size of this vector depends on the number of generator points and the window size.
    table: Vec<blst_p1_affine>,
    /// Window size in bits used for the scalar decomposition in the MSM.
    ///
    /// Determines the number of table entries per generator: `2^wbits`.
    wbits: usize,
    /// Number of generator points used in the precomputation.
    num_points: usize,
    /// Size in words (limbs) of the scratch space required for `blst_p1s_mult_wbits`.
    ///
    /// Needed when calling the MSM function to ensure enough memory is allocated.
    scratch_space_size: usize,
}

/// UsePrecomp indicates whether we should use pre-computations to speed up the MSM
/// and the level of precomputation to perform.
#[derive(Debug, Copy, Clone)]
pub enum UsePrecomp {
    /// Enables fixed-base precomputation with a specified window size (in bits).
    Yes {
        /// Window size in bits.
        width: usize,
    },
    /// Disables fixed-base precomputation.
    No,
}

/// FixedBaseMSM computes a multi scalar multiplication where the points are known beforehand.
///
/// Since the points are known, one can choose to precompute multiple of the points
/// in order to reduce the amount of work needed to compute the MSM, at the cost
/// of memory.
#[derive(Debug)]
pub enum FixedBaseMSM {
    /// Uses a precomputed table for fast fixed-base MSM.
    Precomp(FixedBaseMSMPrecompWindow),
    /// Falls back to regular scalar multiplication without precomputation.
    NoPrecomp(Vec<G1Affine>),
}

impl FixedBaseMSM {
    /// Constructs a `FixedBaseMSM` from a list of fixed generators and a precomputation policy.
    ///
    /// - If `use_precomp` is `Yes`, it builds a precomputed window table for fast fixed-base MSM.
    /// - Otherwise, it stores the generators directly for standard MSM computation.
    pub fn new(generators: Vec<G1Affine>, use_precomp: UsePrecomp) -> Self {
        match use_precomp {
            UsePrecomp::Yes { width } => {
                Self::Precomp(FixedBaseMSMPrecompWindow::new(&generators, width))
            }
            UsePrecomp::No => Self::NoPrecomp(generators),
        }
    }

    /// Computes a multi-scalar multiplication (MSM) using the stored generators and given scalars.
    ///
    /// - If precomputation is enabled, it uses the optimized windowed method;
    /// - Otherwise, it falls back to a standard linear combination.
    ///   Panics if the number of scalars doesn't match the number of generators.
    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        match self {
            Self::Precomp(precomp) => precomp.msm(scalars),
            Self::NoPrecomp(generators) => g1_lincomb(generators, scalars)
                .expect("number of generators and scalars should be equal"),
        }
    }
}

impl FixedBaseMSMPrecompBLST {
    /// Constructs a new `FixedBaseMSMPrecompBLST` instance with precomputed tables
    /// for efficient fixed-base multi-scalar multiplication (MSM).
    ///
    /// # Parameters
    /// - `generators_affine`: A vector of G1 affine points to precompute multiples of.
    /// - `wbits`: The window size in bits for the fixed-base MSM.
    ///
    /// # Returns
    /// A `FixedBaseMSMPrecompBLST` struct containing the precomputed table, window width, and
    /// scratchpad size needed for efficient MSM with these generators.
    ///
    /// # Safety
    /// Unsafe FFI calls to BLST are required to compute and use the precomputation table. These
    /// calls are memory-safe under the assumption that all inputs are valid.
    pub fn new(generators_affine: &[G1Affine], wbits: usize) -> Self {
        // Total number of input points
        let num_points = generators_affine.len();

        // Compute the size of the precomputation table in number of blst_p1_affine elements.
        //
        // The BLST API returns the size in bytes, so we divide by the element size.
        let table_len = unsafe {
            blst::blst_p1s_mult_wbits_precompute_sizeof(wbits, num_points)
                / std::mem::size_of::<blst_p1_affine>()
        };

        // blst expects these to be references, so we convert from Vec<T> to Vec<&T>
        let generators_affine: Vec<&G1Affine> = generators_affine.iter().collect();

        // Convert the slice of references into a pointer-to-pointer format expected by BLST.
        let points = generators_affine
            .as_ptr()
            .cast::<*const blst::blst_p1_affine>();

        // Allocate memory for the precomputed table
        let mut table = vec![blst_p1_affine::default(); table_len];
        // Perform the actual precomputation using BLST.
        //
        // This fills `table` with precomputed multiples of the input points using a width-wbits window.
        unsafe {
            blst::blst_p1s_mult_wbits_precompute(table.as_mut_ptr(), wbits, points, num_points);
        };

        Self {
            table,
            wbits,
            num_points,
            scratch_space_size: unsafe { blst::blst_p1s_mult_wbits_scratch_sizeof(num_points) },
        }
    }

    /// Performs a multi-scalar multiplication using the precomputed table.
    ///
    /// Converts the scalars into BLST-compatible representations and uses
    /// the windowed multiplication routine from BLST with precomputed points.
    #[allow(clippy::ptr_as_ptr)]
    pub fn msm(&self, scalars: Vec<Scalar>) -> G1Projective {
        const NUM_BITS_SCALAR: usize = Scalar::NUM_BITS as usize;

        // Check that the number of scalars matches the number of points
        assert_eq!(
            scalars.len(),
            self.num_points,
            "Number of scalars must match number of points"
        );

        // Convert scalars into raw byte pointers
        let blst_scalars: Vec<_> = scalars
            .into_iter()
            .map(|scalar| Into::<blst::blst_scalar>::into(scalar).b)
            .collect();
        let blst_scalar_ptrs: Vec<*const u8> = blst_scalars
            .iter()
            .map(|s| s as *const _ as *const u8)
            .collect();

        // Prepare scratch space and output
        let mut ret = blst::blst_p1::default();
        let mut scratch_pad: Vec<_> = Vec::with_capacity(self.scratch_space_size);

        // Perform MSM using BLST
        unsafe {
            blst::blst_p1s_mult_wbits(
                &mut ret,
                self.table.as_ptr(),
                self.wbits,
                self.num_points,
                blst_scalar_ptrs.as_ptr(),
                NUM_BITS_SCALAR,
                scratch_pad.as_mut_ptr(),
            );
        }

        // Convert result from BLST to blstrs
        G1Projective::from_raw_unchecked(
            Fp::from_raw_unchecked(ret.x.l),
            Fp::from_raw_unchecked(ret.y.l),
            Fp::from_raw_unchecked(ret.z.l),
        )
    }
}

#[cfg(test)]
mod tests {
    use ff::Field;
    use group::Group;
    use rand::{rngs::StdRng, thread_rng, SeedableRng};

    use super::*;

    fn random_g1_affines(n: usize) -> Vec<G1Affine> {
        let mut rng = StdRng::seed_from_u64(42);
        (0..n)
            .map(|_| G1Projective::random(&mut rng).into())
            .collect()
    }

    fn random_scalars(n: usize) -> Vec<Scalar> {
        let mut rng = StdRng::seed_from_u64(1337);
        (0..n).map(|_| Scalar::random(&mut rng)).collect()
    }

    fn test_fixed_base_msm_with_precomp(use_precomp: UsePrecomp) {
        let length = 64;
        let generators: Vec<_> = (0..length)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();
        let scalars: Vec<_> = (0..length)
            .map(|_| Scalar::random(&mut thread_rng()))
            .collect();

        let res = g1_lincomb(&generators, &scalars)
            .expect("number of generators and number of scalars is equal");

        let fbm = FixedBaseMSM::new(generators, use_precomp);
        let result = fbm.msm(&scalars);

        assert_eq!(res, result);
    }

    #[test]
    fn smoke_test_fixed_base_msm() {
        test_fixed_base_msm_with_precomp(UsePrecomp::No);
        test_fixed_base_msm_with_precomp(UsePrecomp::Yes { width: 4 });
        test_fixed_base_msm_with_precomp(UsePrecomp::Yes { width: 8 });
    }

    #[test]
    fn fixed_base_msm_non_zero() {
        // All elements in the table should be non-zero
        let length = 64;
        let generators: Vec<_> = (0..length)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();
        let fbm = FixedBaseMSMPrecompBLST::new(&generators, 8);
        for val in fbm.table {
            let is_inf = unsafe { blst::blst_p1_affine_is_inf(std::ptr::addr_of!(val)) };
            assert!(!is_inf);
        }
    }

    #[test]
    fn msm_all_zero_scalars_returns_identity() {
        let generators = random_g1_affines(8);
        let scalars = vec![Scalar::ZERO; 8];
        let msm = FixedBaseMSMPrecompBLST::new(&generators, 4);
        let result = msm.msm(scalars);
        assert_eq!(result, G1Projective::identity());
    }

    #[test]
    fn msm_matches_naive_scalar_multiplication() {
        let generators = random_g1_affines(16);
        let scalars = random_scalars(16);
        let expected: G1Projective = generators
            .iter()
            .zip(&scalars)
            .map(|(p, s)| G1Projective::from(*p) * s)
            .sum();

        let msm = FixedBaseMSMPrecompBLST::new(&generators, 4);
        let result = msm.msm(scalars);

        assert_eq!(result, expected);
    }

    #[test]
    fn msm_consistent_across_wbits_settings() {
        let generators = random_g1_affines(16);
        let scalars = random_scalars(16);

        let base_result = {
            let msm = FixedBaseMSMPrecompBLST::new(&generators, 4);
            msm.msm(scalars.clone())
        };

        for w in [2, 3, 5, 6, 8] {
            let msm = FixedBaseMSMPrecompBLST::new(&generators, w);
            let result = msm.msm(scalars.clone());
            assert_eq!(result, base_result, "Mismatch for wbits = {w}");
        }
    }

    #[test]
    #[should_panic]
    fn msm_panics_on_mismatched_input_lengths() {
        let generators = random_g1_affines(8);
        let scalars = random_scalars(7); // length mismatch
        let msm = FixedBaseMSMPrecompBLST::new(&generators, 4);
        let _ = msm.msm(scalars); // should panic
    }
}
