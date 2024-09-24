use crate::{
    fixed_base_msm_blst::FixedBaseMSMPrecompBLST,
    fixed_base_msm_blst_all_windows::FixedBaseMSMPrecompAllWindow,
    fixed_base_msm_pippenger::FixedBaseMSMPippenger,
    limlee::{LimLee, TsaurChou},
    G1Projective, Scalar,
};
use blstrs::{Fp, G1Affine};

/// FixedBaseMSMPrecomp computes a multi scalar multiplication using pre-computations.
///
/// It uses batch addition to amortize the cost of adding multiple points together.
#[derive(Debug)]
pub struct FixedBaseMSMPrecomp {
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
    Precomp(FixedBaseMSMPrecompAllWindow),
    // Precomp(LimLee),
    // TODO: We are hijacking the NoPrecomp variant to store the
    // TODO: new pippenger algorithm.
    NoPrecomp(FixedBaseMSMPippenger),
}

impl FixedBaseMSM {
    pub fn new(generators: Vec<G1Affine>, use_precomp: UsePrecomp) -> Self {
        match use_precomp {
            UsePrecomp::Yes { width } => {
                FixedBaseMSM::Precomp(FixedBaseMSMPrecompAllWindow::new(&generators, width))
                // FixedBaseMSM::Precomp(FixedBaseMSMPrecompBLST::new(&generators, width))
                // FixedBaseMSM::Precomp(TsaurChou::new(8, 4, &generators))
                // FixedBaseMSM::Precomp(LimLee::new(8, 1, &generators))
            }
            UsePrecomp::No => FixedBaseMSM::NoPrecomp(FixedBaseMSMPippenger::new(&generators)),
        }
    }

    pub fn msm(&self, scalars: Vec<Scalar>) -> G1Projective {
        match self {
            FixedBaseMSM::Precomp(precomp) => {
                // TsaurChau
                // precomp.mul_naive_better_wnaf_precomputations_final_msm(&scalars)
                precomp.msm(&scalars)
            }
            FixedBaseMSM::NoPrecomp(precomp) => precomp.msm(&scalars),
        }
    }
}

impl FixedBaseMSMPrecomp {
    pub fn new(generators_affine: Vec<G1Affine>, wbits: usize) -> Self {
        let num_points = generators_affine.len();
        let table_size_bytes =
            unsafe { blst::blst_p1s_mult_wbits_precompute_sizeof(wbits, num_points) };

        // blst expects these to be references, so we convert from Vec<T> to Vec<&T>
        let generators_affine: Vec<&G1Affine> = generators_affine.iter().collect();

        // Calculate the number of blst_p1_affine elements
        let table_size = table_size_bytes / std::mem::size_of::<blst::blst_p1_affine>();

        let points = generators_affine.as_ptr() as *const *const blst::blst_p1_affine;

        let mut table = vec![blst::blst_p1_affine::default(); table_size];
        unsafe {
            blst::blst_p1s_mult_wbits_precompute(table.as_mut_ptr(), wbits, points, num_points)
        };

        let scratch_space_size = unsafe { blst::blst_p1s_mult_wbits_scratch_sizeof(num_points) };

        FixedBaseMSMPrecomp {
            table,
            wbits,
            num_points,
            scratch_space_size,
        }
    }

    pub fn msm(&self, scalars: Vec<Scalar>) -> G1Projective {
        use ff::PrimeField;
        let mut ret = blst::blst_p1::default();
        const NUM_BITS_SCALAR: usize = Scalar::NUM_BITS as usize;

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
    use super::{FixedBaseMSMPrecomp, UsePrecomp};
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

        let res = crate::lincomb::g1_lincomb(&generators, &scalars)
            .expect("number of generators and number of scalars is equal");

        let fbm = FixedBaseMSM::new(generators.clone(), use_precomp);
        let result = fbm.msm(scalars);

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
        let fbm = FixedBaseMSMPrecomp::new(generators, 8);
        for val in fbm.table.into_iter() {
            let is_inf =
                unsafe { blst::blst_p1_affine_is_inf(&val as *const blst::blst_p1_affine) };
            assert!(!is_inf);
        }
    }
}
