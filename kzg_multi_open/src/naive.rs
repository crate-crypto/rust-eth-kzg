use bls12_381::{multi_pairings, G1Point, G1Projective, G2Point, G2Prepared, Scalar};
use polynomial::monomial::{lagrange_interpolate, poly_eval, poly_sub, vanishing_poly, PolyCoeff};
use crate::{commit_key::CommitKey, opening_key::OpeningKey, proof::Proof};

/// This modules contains code to create and verify opening proofs in a naive way.
/// It is also generally meaning the points we are creating opening proofs
/// for, do not need to have any special structure.
/// 
/// This generalized scheme can be seen in [BDFG21](https://eprint.iacr.org/2020/081.pdf)
/// 
/// This is in contrast to the scheme we will use in practice which dictates that the
/// points we open at, must be roots of unity. This scheme is called FK20 and is orders
/// of magnitudes faster than the naive scheme.
/// 
/// We will use the naive scheme for testing purposes.

/// Naively computes an opening proof that attests to the evaluation of
/// `polynomial` at `input_points`.
pub fn compute_multi_opening(
    commit_key: &CommitKey,
    polynomial: &PolyCoeff,
    input_points: &[Scalar],
) -> Proof {
    let (quotient_commitment, output_points) =
        _compute_multi_opening_naive(commit_key, polynomial, input_points);

    Proof {
        quotient_commitment,
        output_points,
    }
}

/// Naively Verifies a multi-point opening proof.
pub fn verify_multi_opening(
    proof : &Proof,
    opening_key: &OpeningKey,
    commitment: G1Point,
    input_points: &[Scalar],
) -> bool {
    _verify_multi_opening_naive(
        opening_key,
        commitment,
        proof.quotient_commitment,
        input_points,
        &proof.output_points,
    )
}


/// Computes a multi-point opening proof using the general formula.
///
/// Note: This copies the implementation that the consensus-specs uses.
/// With the exception that division is done using Ruffini's rule.
fn _compute_multi_opening_naive(
    commit_key: &CommitKey,
    polynomial: &PolyCoeff,
    points: &[Scalar],
) -> (G1Point, Vec<Scalar>) {
    // Divides `self` by x-z using Ruffinis rule
    fn divide_by_linear(poly: &[Scalar], z: Scalar) -> Vec<Scalar> {
        let mut quotient: Vec<Scalar> = Vec::with_capacity(poly.len());
        let mut k = Scalar::from(0u64);

        for coeff in poly.iter().rev() {
            let t = *coeff + k;
            quotient.push(t);
            k = z * t;
        }

        // Pop off the remainder term
        quotient.pop();

        // Reverse the results as monomial form stores coefficients starting with lowest degree
        quotient.reverse();
        quotient
    }

    let mut evaluations = Vec::new();
    for point in points {
        let evaluation = poly_eval(polynomial, point);
        evaluations.push(evaluation);
    }

    // Compute f(x) - I(x) / \prod (x - z_i)
    // Where I(x) is the polynomial such that r(z_i) = f(z_i) for all z_i
    //
    // We can speed up computation of I(x) by doing an IFFT, given the coset generator, since
    // we know all of the points are of the form k * \omega where \omega is a root of unity

    let coordinates: Vec<_> = points
        .iter()
        .zip(evaluations.iter())
        .map(|(p, e)| (*p, *e))
        .collect();

    let r_x = lagrange_interpolate(&coordinates).unwrap();

    // check that the r_x polynomial is correct, it should essentially be the polynomial that
    // evaluates to f(z_i) = r(z_i)
    for (point, evaluation) in points.iter().zip(evaluations.iter()) {
        assert_eq!(poly_eval(&r_x, point), *evaluation);
    }

    let poly_shifted = poly_sub(polynomial.to_vec().clone(), r_x.clone());

    let mut quotient_poly = poly_shifted.to_vec().clone();
    for point in points.iter() {
        quotient_poly = divide_by_linear(&quotient_poly, *point);
    }

    (commit_key.commit_g1(&quotient_poly).into(), evaluations)
}


/// Verifies a multi-opening proof using the general formula.
///
/// Note: This copies the exact implementation that the consensus-specs uses.
fn _verify_multi_opening_naive(
    opening_key: &OpeningKey,
    commitment: G1Point,
    proof: G1Point,
    input_points: &[Scalar],
    output_points: &[Scalar],
) -> bool {
    // e([Commitment] - [I(x)], [1]) == e([Q(x)], [Z(X)])

    let coordinates: Vec<_> = input_points
        .iter()
        .zip(output_points.iter())
        .map(|(p, e)| (*p, *e))
        .collect();
    let r_x = lagrange_interpolate(&coordinates).unwrap();

    let vanishing_poly = vanishing_poly(input_points);
    let comm_vanishing_poly: G2Point = opening_key.commit_g2(&vanishing_poly).into();

    let comm_r_x = opening_key.commit_g1(&r_x);
    let comm_minus_r_x: G1Point = (G1Projective::from(commitment) - comm_r_x).into();
    multi_pairings(&[
        (&proof, &G2Prepared::from(comm_vanishing_poly)),
        (&comm_minus_r_x, &G2Prepared::from(-opening_key.g2_gen())),
    ])
}

#[cfg(test)]
mod tests {
    use bls12_381::Scalar;

    use crate::create_eth_commit_opening_keys;


#[test]
fn smoke_test_naive_multi_opening() {
    let (ck, opening_key) = create_eth_commit_opening_keys();

    let num_points_to_open = 16;
    let input_points : Vec<_> = (0..num_points_to_open).map(|i| Scalar::from(i)).collect();
    
    let polynomial : Vec<_> = (0..opening_key.num_coefficients_in_polynomial).map(|i| -Scalar::from(i as u64)).collect();
    let commitment = ck.commit_g1(&polynomial).into();
    
    
    let proof = super::compute_multi_opening(&ck, &polynomial,&input_points);
    let proof_valid = super::verify_multi_opening(&proof, &opening_key, commitment, &input_points);
    assert!(proof_valid);
    
    // Proof is invalid since we changed the input points
    let input_points : Vec<_> = (0..num_points_to_open).map(|i| Scalar::from(i) + Scalar::from(i)).collect();
    let proof_valid = super::verify_multi_opening(&proof, &opening_key, commitment, &input_points);
    assert!(!proof_valid);
    
}

}