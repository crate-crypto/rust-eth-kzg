use bls12_381::Scalar;

/// This file will hold the implementation of a polynomial in monomial form

/// A polynomial in monomial form where the lowest degree term is first
/// Layout: x^0 * a_0 + x^1 * a_1 + ... + x^(n-1) * a_(n-1)
pub type PolyCoeff = Vec<Scalar>;

/// For two polynomials, `f(x)` and `g(x)`, this method computes 
/// the result of `f(x) + g(x)` and returns the result.
/// 
/// Note: Polynomials can be of different lengths.
pub fn poly_add(a: PolyCoeff, b: PolyCoeff) -> PolyCoeff {
    let (smaller_poly, mut larger_poly) = if a.len() < b.len() { (a, b) } else { (b, a) };

    for i in 0..smaller_poly.len() {
        larger_poly[i] += smaller_poly[i];
    }

    larger_poly
}

/// For a polynomial, `f(x)`, this method computes the result of `-f(x)`
/// and returns the result.
pub fn poly_neg(mut a: PolyCoeff) -> PolyCoeff {
    for i in 0..a.len() {
        a[i] = -a[i];
    }
    a
}

/// For two polynomials, `f(x)` and `g(x)`, this method computes
/// the result of `f(x) - g(x)` and returns the result.
/// 
/// Note: Polynomials can be of different lengths
pub fn poly_sub(a: PolyCoeff, b: PolyCoeff) -> PolyCoeff {
    let neg_b = poly_neg(b);
    poly_add(a, neg_b)
}

/// Computes powers of a scalar up to and including the given degree.
///
/// Example: powers(x, 10) == [1, x, x^2, ..., x^10]
fn powers_of(scalar: &Scalar, max_degree: usize) -> Vec<Scalar> {
    let mut powers = Vec::with_capacity(max_degree);
    powers.push(Scalar::from(1u64));
    for i in 1..=max_degree {
        powers.push(powers[i - 1] * scalar);
    }
    powers
}

/// Given a polynomial `f(x)` and a scalar `z`. This method will compute
/// the result of `f(z)` and return the result.
pub fn poly_eval(poly: &PolyCoeff, value: &Scalar) -> Scalar {
    // If the scalar, we are evaluating at it zero, return the constant term
    if value == &Scalar::from(0u64) {
        return poly[0];
    }

    let powers = powers_of(value, poly.len());

    let mut sum = Scalar::from(0u64);
    for (power, coeff) in powers.iter().zip(poly.iter()) {
        sum += coeff * power
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use bls12_381::ff::Field;

    #[test]
    fn basic_polynomial_add() {
        let a = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        let b = vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)];
        let c = vec![Scalar::from(5), Scalar::from(7), Scalar::from(9)];
        assert_eq!(poly_add(a, b), c);

        let a = vec![Scalar::from(2), Scalar::from(3)];
        let b = vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)];
        let c = vec![Scalar::from(6), Scalar::from(8), Scalar::from(6)];
        assert_eq!(poly_add(a, b), c);
    }

    #[test]
    fn polynomial_neg() {
        let a = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        let b = vec![-Scalar::from(1), -Scalar::from(2), -Scalar::from(3)];
        assert_eq!(poly_neg(a), b);
    }

    #[test]
    fn basic_polynomial_subtraction() {
        let a = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        let b = vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)];
        let c = vec![-Scalar::from(3), -Scalar::from(3), -Scalar::from(3)];
        assert_eq!(poly_sub(a, b), c);

        let a = vec![Scalar::from(4), Scalar::from(5)];
        let b = vec![Scalar::from(6), Scalar::from(7), Scalar::from(8)];
        let c = vec![-Scalar::from(2), -Scalar::from(2), -Scalar::from(8)];
        assert_eq!(poly_sub(a, b), c);
    }

    #[test]
    fn powers_of_smoke_test() {
        let scalar = Scalar::from(2u64);
        let max_degree = 10;
        let powers = powers_of(&scalar, max_degree);

        assert_eq!(powers.len(), max_degree + 1);
        
        for i in 0..=max_degree {
            assert_eq!(powers[i], scalar.pow_vartime(&[i as u64]));
        }
    }

    #[test]
    fn polynomial_evaluation() {
        // f(x) = 1 + 2x + 3x^2
        // f(2) = 1 + 2*2 + 3*2^2 = 1 + 4 + 12 = 17
        let poly = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        let value = Scalar::from(2u64);
        let result = Scalar::from(17u64);
        assert!(poly_eval(&poly, &value) == result);

        // f(0) = 1
        let value = Scalar::from(0u64);
        let result = Scalar::from(1u64);
        assert!(poly_eval(&poly, &value) == result);
    }
}