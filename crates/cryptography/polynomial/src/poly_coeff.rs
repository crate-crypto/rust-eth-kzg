use std::ops::{Deref, DerefMut};

use bls12_381::{traits::*, Scalar};

/// A polynomial in monomial form over the field `Scalar`.
///
/// Internally stores coefficients in ascending order of degree:
///
/// ```text
/// Layout: x^0 * a_0 + x^1 * a_1 + ... + x^(n-1) * a_(n-1)
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct PolyCoeff(pub Vec<Scalar>);

impl PolyCoeff {
    /// Adds two polynomials `self + other` and returns the result.
    ///
    /// Polynomials may have different lengths; the shorter one is padded with zeros.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        let mut result = self.clone();
        if other.len() > result.len() {
            result.resize(other.len(), Scalar::ZERO);
        }
        for (i, &b) in other.iter().enumerate() {
            result[i] += b;
        }
        result
    }

    /// Computes the additive inverse `-self` and returns the result.
    #[must_use]
    pub fn neg(&self) -> Self {
        Self(self.iter().map(|c| -*c).collect())
    }

    /// Subtracts `other` from `self`, returning `self - other`.
    ///
    /// Internally implemented as `self + (-other)`.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// Evaluates the polynomial at the given scalar point `x`.
    ///
    /// Uses Horner’s method for efficient evaluation.
    #[must_use]
    pub fn eval(&self, x: &Scalar) -> Scalar {
        let mut result = Scalar::ZERO;
        for coeff in self.iter().rev() {
            result = result * x + coeff;
        }
        result
    }

    /// Multiplies two polynomials `self * other` and returns the result.
    ///
    /// The result has degree `self.degree() + other.degree()`.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        let mut result = Self(vec![
            Scalar::ZERO;
            (self.len() + other.len()).saturating_sub(1)
        ]);
        for (i, a) in self.iter().enumerate() {
            for (j, b) in other.iter().enumerate() {
                result[i + j] += a * b;
            }
        }
        result
    }
}

impl Deref for PolyCoeff {
    type Target = Vec<Scalar>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PolyCoeff {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<Scalar>> for PolyCoeff {
    fn from(value: Vec<Scalar>) -> Self {
        Self(value)
    }
}

/// Given a list of points, this method will compute the polynomial
/// Z(x) which is equal to zero when evaluated at each point.
///
/// Example: vanishing_poly([1, 2, 3]) = (x - 1)(x - 2)(x - 3)
pub fn vanishing_poly(roots: &[Scalar]) -> PolyCoeff {
    let mut poly = PolyCoeff(vec![Scalar::ONE]);
    for root in roots {
        poly = poly.mul(&PolyCoeff(vec![-root, Scalar::ONE]));
    }
    poly
}

/// Interpolates a polynomial in monomial form from a list of points (x_i, y_i).
///
/// Uses the classic Lagrange interpolation formula. The result is the unique
/// polynomial of degree < n that passes through all points.
///
/// Time complexity is O(n^2). Intended for small inputs and testing only.
pub fn lagrange_interpolate(points: &[(Scalar, Scalar)]) -> Option<PolyCoeff> {
    // Number of interpolation points. The resulting polynomial has degree < n.
    let n = points.len();

    // Ensure there are at least two points to interpolate
    assert!(n >= 2, "interpolation requires at least 2 points");

    // Initialize the result polynomial to zero: result(x) = 0
    let mut result = vec![Scalar::ZERO; n];

    // Loop over each interpolation point (x_i, y_i)
    for (i, &(x_i, y_i)) in points.iter().enumerate() {
        // Start with the constant polynomial 1 for the Lagrange basis polynomial L_i(x)
        let mut basis = vec![Scalar::ONE];

        // This will accumulate the denominator: product of (x_i - x_j) for j ≠ i
        let mut denom = Scalar::ONE;

        // Construct L_i(x) = product over j ≠ i of (x - x_j)
        for (j, &(x_j, _)) in points.iter().enumerate() {
            if i == j {
                continue;
            }

            // Multiply the denominator by (x_i - x_j)
            denom *= x_i - x_j;

            // Multiply the current basis polynomial by (x - x_j)
            // If basis(x) = a_0 + a_1 * x + ... + a_d * x^d,
            // then basis(x) * (x - x_j) becomes a polynomial of degree d+1:
            //     new_coeff[k] = -x_j * a_k     for x^k
            //     new_coeff[k+1] = a_k          for x^{k+1}
            let mut next = vec![Scalar::ZERO; basis.len() + 1];
            for (k, &coeff_k) in basis.iter().enumerate() {
                next[k] -= coeff_k * x_j;
                next[k + 1] += coeff_k;
            }

            // Replace the basis with the updated polynomial
            basis = next;
        }

        // Compute the scaling factor = y_i / denom
        let scale = y_i * denom.invert().expect("denominator must be non-zero");

        // Add scale * basis(x) to the result polynomial
        for (res_k, basis_k) in result.iter_mut().zip(basis) {
            *res_k += scale * basis_k;
        }
    }

    // Wrap the result in PolyCoeff and return
    Some(PolyCoeff(result))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Small helper function to generate a vector of `Scalar`s
    fn arb_scalar_vec(max_len: usize) -> impl Strategy<Value = Vec<Scalar>> {
        prop::collection::vec(any::<u64>().prop_map(Scalar::from), 0..=max_len)
    }

    fn naive_poly_eval(poly: &PolyCoeff, value: &Scalar) -> Scalar {
        let mut result = Scalar::ZERO;
        for (i, coeff) in poly.iter().enumerate() {
            result += coeff * value.pow_vartime([i as u64]);
        }
        result
    }

    #[test]
    fn basic_polynomial_add() {
        let a = PolyCoeff(vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)]);
        let b = PolyCoeff(vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)]);
        let c = PolyCoeff(vec![Scalar::from(5), Scalar::from(7), Scalar::from(9)]);
        assert_eq!(a.add(&b), c);

        let a = PolyCoeff(vec![Scalar::from(2), Scalar::from(3)]);
        let b = PolyCoeff(vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)]);
        let c = PolyCoeff(vec![Scalar::from(6), Scalar::from(8), Scalar::from(6)]);
        assert_eq!(a.add(&b), c);
    }

    #[test]
    fn polynomial_neg() {
        let a = PolyCoeff(vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)]);
        let b = PolyCoeff(vec![-Scalar::from(1), -Scalar::from(2), -Scalar::from(3)]);
        assert_eq!(a.neg(), b);
    }

    #[test]
    fn basic_polynomial_subtraction() {
        let a = PolyCoeff(vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)]);
        let b = PolyCoeff(vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)]);
        let c = PolyCoeff(vec![-Scalar::from(3), -Scalar::from(3), -Scalar::from(3)]);
        assert_eq!(a.sub(&b), c);

        let a = PolyCoeff(vec![Scalar::from(4), Scalar::from(5)]);
        let b = PolyCoeff(vec![Scalar::from(6), Scalar::from(7), Scalar::from(8)]);
        let c = PolyCoeff(vec![-Scalar::from(2), -Scalar::from(2), -Scalar::from(8)]);
        assert_eq!(a.sub(&b), c);
    }

    #[test]
    fn polynomial_evaluation() {
        // f(x) = 1 + 2x + 3x^2
        // f(2) = 1 + 2*2 + 3*2^2 = 1 + 4 + 12 = 17
        let poly = PolyCoeff(vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)]);
        let value = Scalar::from(2u64);
        assert!(poly.eval(&value) == naive_poly_eval(&poly, &value));
    }

    #[test]
    fn polynomial_multiplication() {
        // f(x) = 1 + 2x + 3x^2
        // g(x) = 4 + 5x
        // f(x) * g(x) = 4 + 8x + 12x^2 + 5x + 10x^2 + 15x^3 = 4 + 13x + 22x^2 + 15x^3
        let a = PolyCoeff(vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)]);
        let b = PolyCoeff(vec![Scalar::from(4), Scalar::from(5)]);
        let expected = PolyCoeff(vec![
            Scalar::from(4),
            Scalar::from(13),
            Scalar::from(22),
            Scalar::from(15),
        ]);
        assert_eq!(a.mul(&b), expected);
    }

    #[test]
    fn vanishing_polynomial_smoke_test() {
        // f(x) = (x - 1)(x - 2)(x - 3) = x^3 - 6x^2 + 11x - 6
        let roots = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let expected = PolyCoeff(vec![
            -Scalar::from(6u64),
            Scalar::from(11u64),
            -Scalar::from(6u64),
            Scalar::from(1u64),
        ]);
        let poly = vanishing_poly(&roots);
        assert_eq!(&poly, &expected);

        // Check that this polynomial evaluates to zero on the roots
        for root in &roots {
            assert_eq!(poly.eval(root), Scalar::ZERO);
        }
    }

    #[test]
    fn polynomial_interpolation_smoke_test() {
        // f(x) = 1 + 2x + 3x^2
        // f(0) = 1, f(1) = 6, f(2) = 17
        let points = vec![
            (Scalar::from(0u64), Scalar::from(1u64)),
            (Scalar::from(1u64), Scalar::from(6u64)),
            (Scalar::from(2u64), Scalar::from(17u64)),
        ];
        let poly =
            lagrange_interpolate(&points).expect("enough values were provided for interpolation");
        let expected = PolyCoeff(vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
        ]);
        assert_eq!(poly, expected);
    }

    proptest! {
        #[test]
        fn prop_add_commutative(a in arb_scalar_vec(16), b in arb_scalar_vec(16)) {
            let a_poly = PolyCoeff(a);
            let b_poly = PolyCoeff(b);
            prop_assert_eq!(a_poly.add(&b_poly), b_poly.add(&a_poly));
        }

        #[test]
        fn prop_eval_horner_vs_naive(poly in arb_scalar_vec(12), x in any::<u64>()) {
            let poly = PolyCoeff(poly);
            let x = Scalar::from(x);
            let mut expected = Scalar::ZERO;
            for (i, coeff) in poly.iter().enumerate() {
                expected += coeff * x.pow_vartime([i as u64]);
            }
            prop_assert_eq!(poly.eval(&x), expected);
        }

        #[test]
        fn prop_neg_neg_identity(poly in arb_scalar_vec(12)) {
            let p = PolyCoeff(poly);
            prop_assert_eq!(p.neg().neg(), p);
        }

        #[test]
        fn prop_distributivity(
            a in arb_scalar_vec(8),
            b in arb_scalar_vec(8),
            c in arb_scalar_vec(8),
        ) {
            let a_poly = PolyCoeff(a);
            let b_poly = PolyCoeff(b);
            let c_poly = PolyCoeff(c);

            let left = a_poly.add(&b_poly).mul(&c_poly);
            let right = a_poly.mul(&c_poly).add(&b_poly.mul(&c_poly));

            prop_assert_eq!(left, right);
        }
    }
}
