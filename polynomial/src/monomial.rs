use bls12_381::Scalar;

/// This file will hold the implementation of a polynomial in monomial form

/// A polynomial in monomial form where the lowest degree term is first
/// Layout: x^0 * a_0 + x^1 * a_1 + ... + x^(n-1) * a_(n-1)
pub type PolyCoeff = Vec<Scalar>;

/// For two polynomials, f(x) and g(x), this method computes 
/// the result of `f(x) + g(x)` and returns the result.
/// 
/// Note: Polynomials can be of different lengths
pub fn poly_add(a: PolyCoeff, b: PolyCoeff) -> PolyCoeff {
    let (smaller_poly, mut larger_poly) = if a.len() < b.len() { (a, b) } else { (b, a) };

    for i in 0..smaller_poly.len() {
        larger_poly[i] += smaller_poly[i];
    }

    larger_poly
}

/// For a polynomial, f(x), this method computes the result of `-f(x)`
/// and returns the result.
pub fn poly_neg(mut a: PolyCoeff) -> PolyCoeff {
    for i in 0..a.len() {
        a[i] = -a[i];
    }
    a
}

/// For two polynomials, f(x) and g(x), this method computes
/// the result of `f(x) - g(x)` and returns the result.
/// 
/// Note: Polynomials can be of different lengths
pub fn poly_sub(a: PolyCoeff, b: PolyCoeff) -> PolyCoeff {
    let neg_b = poly_neg(b);
    poly_add(a, neg_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    pub fn basic_polynomial_add() {
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
    pub fn polynomial_neg() {
        let a = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        let b = vec![-Scalar::from(1), -Scalar::from(2), -Scalar::from(3)];
        assert_eq!(poly_neg(a), b);
    }

    #[test]
    pub fn basic_polynomial_subtraction() {
        let a = vec![Scalar::from(1), Scalar::from(2), Scalar::from(3)];
        let b = vec![Scalar::from(4), Scalar::from(5), Scalar::from(6)];
        let c = vec![-Scalar::from(3), -Scalar::from(3), -Scalar::from(3)];
        assert_eq!(poly_sub(a, b), c);

        let a = vec![Scalar::from(4), Scalar::from(5)];
        let b = vec![Scalar::from(6), Scalar::from(7), Scalar::from(8)];
        let c = vec![-Scalar::from(2), -Scalar::from(2), -Scalar::from(8)];
        assert_eq!(poly_sub(a, b), c);
    }
}