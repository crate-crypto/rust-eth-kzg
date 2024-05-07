use std::cell::RefCell;
use blstrs::{Fp, G1Affine};
use crate::{G1Projective, Scalar};

/// FixedBasedMSM computes a multi scalar multiplication by precomputing a table of points.
/// It uses batch addition to amortize the cost of adding these points together.
pub struct FixedBaseMSM {
    table: Vec<blst::blst_p1_affine>,
    wbits: usize,
    num_points: usize,
    scratch_space: RefCell<Vec<blst::limb_t>>,
}

impl FixedBaseMSM {
    pub fn new(generators: Vec<G1Projective>, wbits: usize) -> Self {
        let num_points = generators.len();
        let table_size = unsafe { blst::blst_p1s_mult_wbits_precompute_sizeof(8, num_points) };

        use group::prime::PrimeCurveAffine;
        use group::Curve;
        let mut generators_affine = vec![G1Affine::identity(); generators.len()];
        G1Projective::batch_normalize(&generators, &mut generators_affine);

        // blst expects these to be references, so we convert from Vec<T> to Vec<&T>
        let generators_affine: Vec<&G1Affine> = generators_affine.iter().collect();

        let points = generators_affine.as_ptr() as *const *const blst::blst_p1_affine;

        let mut table = Vec::with_capacity(table_size);
        unsafe {
            blst::blst_p1s_mult_wbits_precompute(table.as_mut_ptr(), wbits, points, num_points)
        };

        let scratch_size = unsafe { blst::blst_p1s_mult_wbits_scratch_sizeof(num_points) };

        FixedBaseMSM {
            table,
            wbits,
            num_points,
            scratch_space: RefCell::new(Vec::with_capacity(scratch_size)),
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

        let mut scratch_pad = self.scratch_space.borrow_mut();

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
            .map(|_| G1Projective::random(&mut rand::thread_rng()))
            .collect();
        let scalars: Vec<_> = (0..length)
            .map(|_| Scalar::random(&mut thread_rng()))
            .collect();

        let res = g1_lincomb(&generators, &scalars);
        let fbm = FixedBaseMSM::new(generators, 8);

        let result = fbm.msm(scalars);
        assert_eq!(res, result);
    }
}
