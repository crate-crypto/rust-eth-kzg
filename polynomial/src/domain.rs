use crate::monomial::PolyCoeff;
use bls12_381::ff::{Field, PrimeField};
use bls12_381::{
    group::Group,
    {G1Projective, Scalar},
};

/// A struct representing a set of points that are roots of unity,
/// which allows us to efficiently evaluate and interpolate polynomial
/// over these points.
#[derive(Debug, Clone)]
pub struct Domain {
    // roots of unity
    pub roots: Vec<Scalar>,
    // Domain size as a scalar
    pub domain_size: Scalar,
    // Inverse of the domain size as a scalar
    pub domain_size_inv: Scalar,
    // Generator for this domain
    // Element has order `domain_size`
    pub generator: Scalar,
    // Inverse of the generator
    // This is cached for IFFT
    pub generator_inv: Scalar,
    // Element used to generate a coset
    // of the domain
    coset_generator: Scalar,
    // Inverse of the coset generator
    coset_generator_inv: Scalar,
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

        Self {
            roots,
            domain_size: size_as_scalar,
            domain_size_inv: size_as_scalar_inv,
            generator,
            generator_inv,
            coset_generator,
            coset_generator_inv,
        }
    }

    /// Computes an n'th root of unity for a given `n`
    ///
    /// TODO: If this shows to be too slow, we can use a lookup table
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
    pub(crate) fn size(&self) -> usize {
        self.roots.len()
    }

    /// Evaluates a polynomial at the domain points.
    pub fn fft_scalars(&self, mut points: PolyCoeff) -> Vec<Scalar> {
        // pad the points with zeroes
        points.resize(self.size(), Scalar::ZERO);
        fft_scalar(self.generator, &points)
    }

    /// Evaluates a polynomial at the points in the domain multiplied by a coset
    /// generator `g`.
    pub fn coset_fft_scalars(&self, mut points: PolyCoeff) -> Vec<Scalar> {
        // pad the points with zeroes
        points.resize(self.size(), Scalar::ZERO);

        let mut coset_scale = Scalar::ONE;
        for point in points.iter_mut() {
            *point *= coset_scale;
            coset_scale *= self.coset_generator;
        }
        fft_scalar(self.generator, &points)
    }

    /// Computes a DFT for the group elements(points) using the domain roots.
    pub fn fft_g1(&self, mut points: Vec<G1Projective>) -> Vec<G1Projective> {
        // pad the points with zeroes
        points.resize(self.size(), G1Projective::identity());
        fft_g1(self.generator, &points)
    }

    /// Computes an IDFT for the group elements(points) using the domain roots.
    pub fn ifft_g1(&self, mut points: Vec<G1Projective>) -> Vec<G1Projective> {
        // pad the points with zeroes
        points.resize(self.size(), G1Projective::identity());

        let mut ifft_g1 = fft_g1(self.generator_inv, &points);

        for element in ifft_g1.iter_mut() {
            *element *= self.domain_size_inv
        }

        ifft_g1
    }

    /// Interpolates the points over the domain to get a polynomial
    /// in monomial form.
    pub fn ifft_scalars(&self, mut points: Vec<Scalar>) -> Vec<Scalar> {
        if points.len() != self.size() {
            panic!(
                "number of points {}, must equal the domain size {}",
                points.len(),
                self.size()
            )
        }

        // pad the points with zeroes
        points.resize(self.size(), Scalar::ZERO);

        let mut ifft_scalar = fft_scalar(self.generator_inv, &points);

        for element in ifft_scalar.iter_mut() {
            *element *= self.domain_size_inv
        }

        ifft_scalar
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

/// Computes a DFT using the given points and the nth root of unity.
fn fft_scalar(nth_root_of_unity: Scalar, points: &[Scalar]) -> Vec<Scalar> {
    let n = points.len();
    if n == 1 {
        return points.to_vec();
    }

    let (even, odd) = take_even_odd(points);

    // Compute a root with half the order
    let gen_squared = nth_root_of_unity.square();

    let fft_even = fft_scalar(gen_squared, &even);
    let fft_odd = fft_scalar(gen_squared, &odd);

    let mut input_point = Scalar::ONE;
    let mut evaluations = vec![Scalar::ONE; n];

    for k in 0..n / 2 {
        let tmp = fft_odd[k] * input_point;
        evaluations[k] = fft_even[k] + tmp;
        evaluations[k + n / 2] = fft_even[k] - tmp;

        input_point *= nth_root_of_unity;
    }

    evaluations
}

/// Computes a DFT of the group elements(points) using powers of the roots of unity.
///
/// Note: This is essentially multiple multi-scalar multiplications.
///
/// TODO: Optimize this, currently very slow -- using split-radix or radix-4 might reduce
/// TODO: the number of multiplications.
fn fft_g1(nth_root_of_unity: Scalar, points: &[G1Projective]) -> Vec<G1Projective> {
    let n = points.len();
    if n == 1 {
        return points.to_vec();
    }

    let (even, odd) = take_even_odd(points);

    // Compute a root with half the order
    let gen_squared = nth_root_of_unity.square();

    let fft_even = fft_g1(gen_squared, &even);
    let fft_odd = fft_g1(gen_squared, &odd);

    let mut input_point = Scalar::ONE;
    let mut evaluations = vec![G1Projective::identity(); n];

    for k in 0..n / 2 {
        let tmp = fft_odd[k] * input_point;
        evaluations[k] = G1Projective::from(fft_even[k]) + tmp;
        evaluations[k + n / 2] = fft_even[k] - tmp;

        input_point *= nth_root_of_unity;
    }

    evaluations
}

/// Splits the list into two lists, one containing the even indexed elements
/// and the other containing the odd indexed elements.
fn take_even_odd<T: Clone>(list: &[T]) -> (Vec<T>, Vec<T>) {
    let mut even = Vec::with_capacity(list.len() / 2);
    let mut odd = Vec::with_capacity(list.len() / 2);

    for (index, value) in list.iter().enumerate() {
        if index % 2 == 0 {
            even.push(value.clone())
        } else {
            odd.push(value.clone())
        }
    }

    (even, odd)
}

#[cfg(test)]
mod tests {
    use crate::monomial::poly_eval;

    use super::*;

    #[test]
    fn take_even_odd_smoke_test() {
        let list = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let (even, odd) = take_even_odd(&list);

        let expected_even_list = vec![1, 3, 5, 7];
        let expected_odd_list = vec![2, 4, 6, 8];

        assert_eq!(even, expected_even_list);
        assert_eq!(odd, expected_odd_list);
    }

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
        let polynomial = vec![Scalar::random(&mut rand::thread_rng()); 32];

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
            powers.push(Scalar::from(1u64));
            for i in 1..=max_degree {
                powers.push(powers[i - 1] * scalar);
            }
            powers
        }

        let n = 4;
        let domain = Domain::new(n);
        let points = vec![G1Projective::random(&mut rand::thread_rng()); n];

        let dft_points = domain.fft_g1(points.clone());
        for (i, root) in domain.roots.iter().enumerate() {
            let powers = powers_of(root, points.len());

            let expected = naive_msm(&points, &powers);
            let got = dft_points[i];
            assert_eq!(expected, got);
        }

        assert_eq!(domain.ifft_g1(dft_points), points);
    }
}
