use crate::{G1Projective, Scalar};
use blstrs::{Fp, G1Affine};

/// FixedBasedMSM computes a multi scalar multiplication using pre-computations.
///
/// It uses batch addition to amortize the cost of adding multiple points together.
#[derive(Debug)]
pub struct FixedBaseMSM {
    table: Vec<blst::blst_p1_affine>,
    wbits: usize,
    num_points: usize,
    scratch_space_size: usize,
}

impl FixedBaseMSM {
    pub fn new(generators_affine: Vec<G1Affine>, wbits: usize) -> Self {
        let num_points = generators_affine.len();
        let table_size = unsafe { blst::blst_p1s_mult_wbits_precompute_sizeof(wbits, num_points) };
        // blst expects these to be references, so we convert from Vec<T> to Vec<&T>
        let generators_affine: Vec<&G1Affine> = generators_affine.iter().collect();

        let points = generators_affine.as_ptr() as *const *const blst::blst_p1_affine;

        let mut table = vec![blst::blst_p1_affine::default(); table_size];
        unsafe {
            blst::blst_p1s_mult_wbits_precompute(table.as_mut_ptr(), wbits, points, num_points)
        };

        let scratch_space_size = unsafe { blst::blst_p1s_mult_wbits_scratch_sizeof(num_points) };

        FixedBaseMSM {
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
    use super::FixedBaseMSM;
    use crate::{lincomb::g1_lincomb, G1Projective, Scalar};
    use ff::Field;
    use group::Group;
    use rand::thread_rng;

    #[test]
    fn smoke_test_fixed_base_msm() {
        let length = 64;
        let generators: Vec<_> = (0..length)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();
        let scalars: Vec<_> = (0..length)
            .map(|_| Scalar::random(&mut thread_rng()))
            .collect();

        let res = g1_lincomb(&generators, &scalars)
            .expect("number of generators and number of scalars is equal");
        let fbm = FixedBaseMSM::new(generators, 8);

        let result = fbm.msm(scalars);
        assert_eq!(res, result);
    }

    #[test]
    fn fixed_base_msm_non_zero() {
        // All elements in the table should be non-zero
        let length = 64;
        let generators: Vec<_> = (0..length)
            .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
            .collect();
        let fbm = FixedBaseMSM::new(generators, 8);
        for val in fbm.table.into_iter() {
            let is_inf =
                unsafe { blst::blst_p1_affine_is_inf(&val as *const blst::blst_p1_affine) };
            assert!(!is_inf);
        }
    }
}
