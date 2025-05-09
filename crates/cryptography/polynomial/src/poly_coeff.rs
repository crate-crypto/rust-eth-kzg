use std::ops::{Deref, DerefMut};

use bls12_381::{ff::Field, Scalar};

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
        result.truncate_leading_zeros();
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

        result.truncate_leading_zeros();
        result
    }

    /// Truncate the polynomial to remove trailing zeros.
    fn truncate_leading_zeros(&mut self) {
        while self.last().is_some_and(|c| c.is_zero().into()) {
            self.pop();
        }
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

/// Interpolates a set of points to a given polynomial in monomial form.
///
/// Given a list of points (x_i, y_i), this method will return the lowest degree polynomial
/// in monomial form that passes through all the points.
///
///
/// A simple O(n^2) algorithm (lagrange interpolation)
///
/// Note: This method is only used for testing. Our domain will always be the roots
/// of unity, so we use IFFT to interpolate.
pub fn lagrange_interpolate(points: &[(Scalar, Scalar)]) -> Option<PolyCoeff> {
    let max_degree_plus_one = points.len();
    assert!(
        max_degree_plus_one >= 2,
        "should interpolate for degree >= 1"
    );
    let mut coeffs = vec![Scalar::ZERO; max_degree_plus_one];
    // external iterator
    for (k, p_k) in points.iter().enumerate() {
        let (x_k, y_k) = p_k;
        // coeffs from 0 to max_degree - 1
        let mut contribution = vec![Scalar::ZERO; max_degree_plus_one];
        let mut denominator = Scalar::ONE;
        let mut max_contribution_degree = 0;
        // internal iterator
        for (j, p_j) in points.iter().enumerate() {
            let (x_j, _) = p_j;
            if j == k {
                continue;
            }

            let mut diff = *x_k;
            diff -= x_j;
            denominator *= diff;

            if max_contribution_degree == 0 {
                max_contribution_degree = 1;
                *contribution
                    .get_mut(0)
                    .expect("must have enough coefficients") -= x_j;
                *contribution
                    .get_mut(1)
                    .expect("must have enough coefficients") += Scalar::from(1u64);
            } else {
                let mul_by_minus_x_j: Vec<Scalar> = contribution
                    .iter()
                    .map(|el| {
                        let mut tmp = *el;
                        tmp *= x_j;

                        -tmp
                    })
                    .collect();

                contribution.insert(0, Scalar::ZERO);
                contribution.truncate(max_degree_plus_one);

                assert_eq!(mul_by_minus_x_j.len(), max_degree_plus_one);
                for (i, c) in contribution.iter_mut().enumerate() {
                    let other = mul_by_minus_x_j
                        .get(i)
                        .expect("should have enough elements");
                    *c += other;
                }
            }
        }

        denominator = denominator
            .invert()
            .expect("unexpected zero in denominator");
        for (i, this_contribution) in contribution.into_iter().enumerate() {
            let c = coeffs.get_mut(i).expect("should have enough coefficients");
            let mut tmp = this_contribution;
            tmp *= denominator;
            tmp *= y_k;
            *c += tmp;
        }
    }

    Some(coeffs.into())
}

#[cfg(test)]
mod tests {
    use bls12_381::ff::Field;
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

    #[test]
    fn test_add_sub_empty() {
        let a = PolyCoeff(vec![]);
        let b = PolyCoeff(vec![Scalar::from(0)]);
        assert_eq!(a.add(&b).sub(&b), a);
    }

    #[test]
    fn test_from_vec_all_zeros() {
        let a = vec![Scalar::from(0); 10];
        assert_eq!(PolyCoeff::from(a), PolyCoeff(vec![]));
    }

    proptest! {
        #[test]
        fn prop_add_commutative(a in arb_scalar_vec(16), b in arb_scalar_vec(16)) {
            let a_poly = PolyCoeff(a);
            let b_poly = PolyCoeff(b);
            prop_assert_eq!(a_poly.add(&b_poly), b_poly.add(&a_poly));
        }

        #[test]
        fn prop_add_sub_roundtrip(a in arb_scalar_vec(16), b in arb_scalar_vec(16)) {
            let a_poly = PolyCoeff(a);
            let b_poly = PolyCoeff(b);
            let sum = a_poly.add(&b_poly);
            let back = sum.sub(&b_poly);
            prop_assert_eq!(a_poly, back);
        }

        #[test]
        fn prop_mul_degree(a in arb_scalar_vec(8), b in arb_scalar_vec(8)) {
            let a_poly = PolyCoeff(a.clone());
            let b_poly = PolyCoeff(b.clone());
            let prod = a_poly.mul(&b_poly);
            let expected_degree = a.len().saturating_sub(1) + b.len().saturating_sub(1);
            prop_assert_eq!(prod.len(), if a.is_empty() || b.is_empty() { 0 } else { expected_degree + 1 });
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
