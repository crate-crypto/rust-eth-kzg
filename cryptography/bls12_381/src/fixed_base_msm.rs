use crate::lincomb::g1_lincomb;
use crate::{fixed_base_msm_window::FixedBaseMSMPrecompWindow, G1Projective, Scalar};
use blstrs::{Fp, G1Affine};
use ff::PrimeField;

/// FixedBaseMSMPrecomp computes a multi scalar multiplication using pre-computations.
///
/// It uses batch addition to amortize the cost of adding multiple points together.
#[derive(Debug)]
pub struct FixedBaseMSMPrecompBLST {
    table: Vec<blst::blst_p1_affine>,
    wbits: usize,
    num_points: usize,
    scratch_space_size: usize,
}

/// UsePrecomp indicates whether we should use pre-computations to speed up the MSM
/// and the level of precomputation to perform.
#[derive(Debug, Copy, Clone)]
pub enum UsePrecomp {
    Yes { width: usize },
    No,
}

/// FixedBaseMSM computes a multi scalar multiplication where the points are known beforehand.
///
/// Since the points are known, one can choose to precompute multiple of the points
/// in order to reduce the amount of work needed to compute the MSM, at the cost
/// of memory.
#[derive(Debug)]
pub enum FixedBaseMSM {
    Precomp(FixedBaseMSMPrecompWindow),
    NoPrecomp(Vec<G1Affine>),
}

impl FixedBaseMSM {
    pub fn new(generators: Vec<G1Affine>, use_precomp: UsePrecomp) -> Self {
        match use_precomp {
            UsePrecomp::Yes { width } => {
                Self::Precomp(FixedBaseMSMPrecompWindow::new(&generators, width))
            }
            UsePrecomp::No => Self::NoPrecomp(generators),
        }
    }

    pub fn msm(&self, scalars: &[Scalar]) -> G1Projective {
        match self {
            Self::Precomp(precomp) => precomp.msm(scalars),
            Self::NoPrecomp(generators) => g1_lincomb(generators, scalars)
                .expect("number of generators and scalars should be equal"),
        }
    }
}

impl FixedBaseMSMPrecompBLST {
    pub fn new(generators_affine: &[G1Affine], wbits: usize) -> Self {
        let num_points = generators_affine.len();
        let table_size_bytes =
            unsafe { blst::blst_p1s_mult_wbits_precompute_sizeof(wbits, num_points) };

        // blst expects these to be references, so we convert from Vec<T> to Vec<&T>
        let generators_affine: Vec<&G1Affine> = generators_affine.iter().collect();

        // Calculate the number of blst_p1_affine elements
        let table_size = table_size_bytes / std::mem::size_of::<blst::blst_p1_affine>();

        let points = generators_affine
            .as_ptr()
            .cast::<*const blst::blst_p1_affine>();

        let mut table = vec![blst::blst_p1_affine::default(); table_size];
        unsafe {
            blst::blst_p1s_mult_wbits_precompute(table.as_mut_ptr(), wbits, points, num_points);
        };

        let scratch_space_size = unsafe { blst::blst_p1s_mult_wbits_scratch_sizeof(num_points) };

        Self {
            table,
            wbits,
            num_points,
            scratch_space_size,
        }
    }

    #[allow(clippy::ptr_as_ptr)]
    pub fn msm(&self, scalars: Vec<Scalar>) -> G1Projective {
        const NUM_BITS_SCALAR: usize = Scalar::NUM_BITS as usize;

        let mut ret = blst::blst_p1::default();

        let blst_scalars: Vec<_> = scalars
            .into_iter()
            .map(|scalar| Into::<blst::blst_scalar>::into(scalar).b)
            .collect();
        let blst_scalar_ptrs: Vec<*const u8> = blst_scalars
            .iter()
            .map(|s| s as *const _ as *const u8)
            .collect();

        let mut scratch_pad: Vec<blst::limb_t> = Vec::with_capacity(self.scratch_space_size);

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

        let x = Fp::from_raw_unchecked(ret.x.l);
        let y = Fp::from_raw_unchecked(ret.y.l);
        let z = Fp::from_raw_unchecked(ret.z.l);

        G1Projective::from_raw_unchecked(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixed_base_msm::FixedBaseMSM, G1Projective, Scalar};
    use ff::Field;
    use group::Group;
    use rand::thread_rng;

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
}
