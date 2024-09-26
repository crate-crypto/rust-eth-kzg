use crate::poly_coeff::PolyCoeff;
use bls12_381::ff::{Field, PrimeField};
use bls12_381::{
    group::Group,
    {G1Projective, Scalar},
};

/// A struct representing a set of points that are roots of unity,
/// which allows us to efficiently evaluate and interpolate polynomial
/// over these points using FFT.
#[derive(Debug, Clone)]
pub struct Domain {
    /// roots of unity
    pub roots: Vec<Scalar>,
    /// size of the domain as a scalar
    pub domain_size: Scalar,
    /// Inverse of the domain size as a scalar
    pub domain_size_inv: Scalar,
    /// Generator for this domain
    // Element has order `domain_size`
    pub generator: Scalar,
    /// Inverse of the generator for the domain
    /// This is cached for IFFT
    pub generator_inv: Scalar,
    /// Element used to generate a coset
    /// of the domain
    coset_generator: Scalar,
    /// Inverse of the coset generator
    coset_generator_inv: Scalar,
    /// Precomputed values for the generator to speed up
    /// the forward FFT
    twiddle_factors: Vec<Scalar>,
    /// Precomputed values for the generator to speed up
    /// the backward FFT
    twiddle_factors_inv: Vec<Scalar>,
}

impl Domain {
    pub fn new(size: usize) -> Domain {
        // We are using roots of unity, so the
        // size of the domain will be padded to
        // the next power of two
        let size = if size.is_power_of_two() {
            size
        } else {
            size.next_power_of_two()
        };

        let generator = Self::compute_generator_for_size(size);
        let generator_inv = generator.invert().expect("generator should not be zero");

        let size_as_scalar = Scalar::from(size as u64);
        let size_as_scalar_inv = size_as_scalar.invert().expect("size should not be zero");

        let mut roots = Vec::with_capacity(size);
        roots.push(Scalar::ONE);

        for i in 1..size {
            let prev_root = roots[i - 1];
            roots.push(prev_root * generator)
        }

        let coset_generator = Scalar::MULTIPLICATIVE_GENERATOR;
        let coset_generator_inv = coset_generator
            .invert()
            .expect("coset generator should not be zero");

        let twiddle_factors = precompute_twiddle_factors(&generator, size);
        let twiddle_factors_inv = precompute_twiddle_factors(&generator_inv, size);

        Self {
            roots,
            domain_size: size_as_scalar,
            domain_size_inv: size_as_scalar_inv,
            generator,
            generator_inv,
            coset_generator,
            coset_generator_inv,
            twiddle_factors,
            twiddle_factors_inv,
        }
    }

    /// Computes an n'th root of unity for a given `n`
    fn compute_generator_for_size(size: usize) -> Scalar {
        assert!(size.is_power_of_two());

        let log_size_of_group = size.trailing_zeros();
        if log_size_of_group > Domain::two_adicity() {
            panic!("two adicity is 32 but group size needed is 2^{log_size_of_group}");
        }

        // We now want to compute the generator which has order `size`
        let exponent: u64 = 1 << (Domain::two_adicity() as u64 - log_size_of_group as u64);

        Domain::largest_root_of_unity().pow_vartime([exponent])
    }

    /// The largest root of unity that we can use for the domain
    const fn largest_root_of_unity() -> Scalar {
        Scalar::ROOT_OF_UNITY
    }

    /// The largest power of two that we can use for the domain
    const fn two_adicity() -> u32 {
        32
    }

    /// The size of the domain
    ///
    /// Note: This is always a power of two
    pub(crate) fn size(&self) -> usize {
        self.roots.len()
    }

    /// Evaluates a polynomial at the points in the domain
    pub fn fft_scalars(&self, mut polynomial: PolyCoeff) -> Vec<Scalar> {
        // Pad the polynomial with zeroes, so that it is the same size as the
        // domain.
        polynomial.resize(self.size(), Scalar::ZERO);

        fft_scalar_inplace(&self.twiddle_factors, &mut polynomial);

        polynomial
    }

    /// Evaluates a polynomial at the points in the domain multiplied by a coset
    /// generator `g`.
    pub fn coset_fft_scalars(&self, mut points: PolyCoeff) -> Vec<Scalar> {
        // Pad the polynomial with zeroes, so that it is the same size as the
        // domain.
        points.resize(self.size(), Scalar::ZERO);

        let mut coset_scale = Scalar::ONE;
        for point in points.iter_mut() {
            *point *= coset_scale;
            coset_scale *= self.coset_generator;
        }
        fft_scalar_inplace(&self.twiddle_factors, &mut points);

        points
    }

    /// Computes a FFT for the group elements(elliptic curve points) using the roots in the domain.
    ///
    /// Note: Thinking about an FFT as multiple inner products between powers of the elements
    /// in the domain and the input polynomial makes this easier to visualize.
    pub fn fft_g1(&self, mut points: Vec<G1Projective>) -> Vec<G1Projective> {
        // Pad the vector of points with zeroes, so that it is the same size as the
        // domain.
        points.resize(self.size(), G1Projective::identity());

        fft_g1_inplace(&self.twiddle_factors, &mut points);

        points
    }

    /// Computes an IFFT for the group elements(elliptic curve points) using the roots in the domain.
    pub fn ifft_g1(&self, points: Vec<G1Projective>) -> Vec<G1Projective> {
        self.ifft_g1_take_n(points, None)
    }

    /// Computes an IFFT for the group elements(elliptic curve points) using the roots in the domain.
    ///
    /// `n`:  indicates how many points we would like to return. Passing `None` will return be equivalent
    /// to compute an ifft_g1 and returning as many elements as there are in the domain.
    ///
    /// This is useful for saving computation on the final scalar multiplication that happens after the
    /// initial FFT is done.
    pub fn ifft_g1_take_n(
        &self,
        mut points: Vec<G1Projective>,
        n: Option<usize>,
    ) -> Vec<G1Projective> {
        // Pad the vector with zeroes, so that it is the same size as the
        // domain.
        points.resize(self.size(), G1Projective::identity());

        fft_g1_inplace(&self.twiddle_factors_inv, &mut points);

        // Truncate the result if a value of `n` was supplied.
        let mut ifft_g1 = match n {
            Some(num_to_take) => {
                assert!(num_to_take < points.len());
                points[0..num_to_take].to_vec()
            }
            None => points,
        };

        for element in ifft_g1.iter_mut() {
            *element *= self.domain_size_inv
        }

        ifft_g1
    }

    /// Interpolates the points over the domain to get a polynomial
    /// in monomial form.
    pub fn ifft_scalars(&self, mut points: Vec<Scalar>) -> Vec<Scalar> {
        // Pad the vector with zeroes, so that it is the same size as the
        // domain.
        points.resize(self.size(), Scalar::ZERO);

        fft_scalar_inplace(&self.twiddle_factors_inv, &mut points);

        for element in points.iter_mut() {
            *element *= self.domain_size_inv
        }

        points
    }

    /// Interpolates a polynomial over the coset of a domain
    pub fn coset_ifft_scalars(&self, points: Vec<Scalar>) -> Vec<Scalar> {
        let mut coset_coeffs = self.ifft_scalars(points);

        let mut coset_scale = Scalar::ONE;
        for element in coset_coeffs.iter_mut() {
            *element *= coset_scale;
            coset_scale *= self.coset_generator_inv;
        }
        coset_coeffs
    }
}

/// Computes a FFT of the field elements(scalars).
///
/// Note: This is essentially multiple inner products.
///
/// TODO: This method is still duplicated below
fn fft_scalar_inplace(twiddle_factors: &[Scalar], a: &mut [Scalar]) {
    let n = a.len();
    let log_n = log2_pow2(n);
    assert_eq!(n, 1 << log_n);

    // Bit-reversal permutation
    for k in 0..n {
        let rk = bitreverse(k as u32, log_n) as usize;
        if k < rk {
            a.swap(rk, k);
        }
    }

    let mut m = 1;
    for s in 0..log_n {
        let w_m = twiddle_factors[s as usize];
        for k in (0..n).step_by(2 * m) {
            let mut w = Scalar::ONE;

            for j in 0..m {
                let t = if w == Scalar::ONE {
                    a[k + j + m]
                } else if w == -Scalar::ONE {
                    -a[k + j + m]
                } else {
                    a[k + j + m] * w
                };

                let u = a[k + j];

                a[k + j] = u + t;
                a[k + j + m] = u - t;

                w *= w_m;
            }
        }
        m *= 2;
    }
}

/// Computes a FFT of the group elements(points).
///
/// Note: This is essentially multiple multi-scalar multiplications.
fn fft_g1_inplace(twiddle_factors: &[Scalar], a: &mut [G1Projective]) {
    let n = a.len();
    let log_n = log2_pow2(n);
    assert_eq!(n, 1 << log_n);

    // Bit-reversal permutation
    for k in 0..n {
        let rk = bitreverse(k as u32, log_n) as usize;
        if k < rk {
            a.swap(rk, k);
        }
    }

    let mut m = 1;
    for s in 0..log_n {
        let w_m = twiddle_factors[s as usize];
        for k in (0..n).step_by(2 * m) {
            let mut w = Scalar::ONE;
            for j in 0..m {
                let t = if w == Scalar::ONE {
                    a[k + j + m]
                } else if w == -Scalar::ONE {
                    -a[k + j + m]
                } else if a[k + j + m].is_identity().into() {
                    G1Projective::identity()
                } else {
                    a[k + j + m] * w
                };

                let u = a[k + j];
                a[k + j] = u + t;
                a[k + j + m] = u - t;
                w *= w_m;
            }
        }
        m *= 2;
    }
}

fn bitreverse(mut n: u32, l: u32) -> u32 {
    let mut r = 0;
    for _ in 0..l {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}
fn log2_pow2(n: usize) -> u32 {
    n.trailing_zeros()
}
fn precompute_twiddle_factors<F: Field>(omega: &F, n: usize) -> Vec<F> {
    let log_n = log2_pow2(n);
    (0..log_n)
        .map(|s| omega.pow(&[(n / (1 << (s + 1))) as u64]))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::poly_coeff::poly_eval;

    use super::*;

    #[test]
    fn largest_root_of_unity_has_correct_order() {
        let root = Domain::largest_root_of_unity();
        let order = 2u64.pow(Domain::two_adicity());

        assert_eq!(root.pow_vartime(&[order]), Scalar::ONE);

        // Check that it is indeed a primitive root of unity
        for i in 0..Domain::two_adicity() {
            assert_ne!(root.pow_vartime(&[2u64.pow(i)]), Scalar::ONE);
        }
    }

    #[test]
    fn fft_test_polynomial() {
        let evaluations = vec![Scalar::from(2u64), Scalar::from(4u64)];
        let domain = Domain::new(2);
        let roots = domain.roots.clone();

        // Interpolate the evaluations
        let poly_coeff = domain.ifft_scalars(evaluations.clone());

        // Check interpolation was correct by evalauting the polynomial at the roots
        for (i, root) in roots.iter().enumerate() {
            let eval = poly_eval(&poly_coeff, root);
            assert_eq!(eval, evaluations[i]);
        }

        // Evaluate the polynomial at the domain points
        let got_evals = domain.fft_scalars(poly_coeff.clone());
        assert_eq!(got_evals, evaluations);
    }

    #[test]
    fn test_polynomial_coset_fft() {
        let polynomial: Vec<_> = (0..32).map(|i| -Scalar::from(i)).collect();

        let domain = Domain::new(32);

        let coset_evals = domain.coset_fft_scalars(polynomial.clone());
        let got_poly = domain.coset_ifft_scalars(coset_evals);

        assert_eq!(got_poly, polynomial);
    }

    #[test]
    fn fft_g1_smoke_test() {
        fn naive_msm(points: &[G1Projective], scalars: &[Scalar]) -> G1Projective {
            let mut acc = G1Projective::identity();
            for (point, scalar) in points.iter().zip(scalars.iter()) {
                acc += point * scalar;
            }
            acc
        }
        fn powers_of(scalar: &Scalar, max_degree: usize) -> Vec<Scalar> {
            let mut powers = Vec::new();
            powers.push(Scalar::ONE);
            for i in 1..=max_degree {
                powers.push(powers[i - 1] * scalar);
            }
            powers
        }

        let n = 4;
        let domain = Domain::new(n);
        let points: Vec<_> = (0..n)
            .map(|_| G1Projective::random(&mut rand::thread_rng()))
            .collect();

        let fft_points = domain.fft_g1(points.clone());
        for (i, root) in domain.roots.iter().enumerate() {
            let powers = powers_of(root, points.len());

            let expected = naive_msm(&points, &powers);
            let got = fft_points[i];
            assert_eq!(expected, got);
        }

        assert_eq!(domain.ifft_g1(fft_points), points);
    }
}
