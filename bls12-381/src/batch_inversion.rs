use crate::Scalar;
use ff::Field;

/// Batch inversion of multiple elements
/// This method will panic if one of the elements is zero
pub fn batch_inverse(elements: &mut [Scalar]) {
    batch_inversion(elements)
}

/// Batch inversion of multiple elements.
/// This method will set the inverse of zero elements to zero.
pub fn batch_inverse_check_for_zero(elements: &mut [Scalar]) {
    let mut zeroes = Vec::new();
    let mut non_zero_vec = Vec::with_capacity(elements.len());
    for i in 0..elements.len() {
        if elements[i].is_zero_vartime() {
            zeroes.push(i);
        } else {
            non_zero_vec.push(elements[i]);
        }
    }
    batch_inverse(&mut non_zero_vec);

    // Now we put back in the zeroes
    for i in 0..elements.len() {
        if !zeroes.contains(&i) {
            elements[i] = non_zero_vec.remove(0);
        }
    }
}

// Taken from arkworks codebase
// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
#[cfg(feature = "rayon")]
fn batch_inversion(v: &mut [Scalar]) {
    // Divide the vector v evenly between all available cores
    let min_elements_per_thread = 1;
    let num_cpus_available = rayon::current_num_threads();
    let num_elems = v.len();
    let num_elem_per_thread =
        std::cmp::max(num_elems / num_cpus_available, min_elements_per_thread);

    // Batch invert in parallel, without copying the vector
    v.par_chunks_mut(num_elem_per_thread).for_each(|mut chunk| {
        serial_batch_inversion(&mut chunk);
    });
}

#[cfg(not(feature = "rayon"))]
fn batch_inversion(v: &mut [Scalar]) {
    serial_batch_inversion(v);
}

/// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
/// This method is explicitly single core.
fn serial_batch_inversion(v: &mut [Scalar]) {
    use std::ops::MulAssign;

    // Montgomery’s Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2
    // but with an optimization to multiply every element in the returned vector by coeff

    // First pass: compute [a, ab, abc, ...]
    let mut prod = Vec::with_capacity(v.len());
    let mut tmp = Scalar::ONE;
    for f in v.iter().filter(|f| !f.is_zero_vartime()) {
        tmp.mul_assign(f);
        prod.push(tmp);
    }

    assert_eq!(prod.len(), v.len(), "inversion by zero is not allowed");

    // Invert `tmp`.
    tmp = tmp.invert().unwrap(); // Guaranteed to be nonzero.

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v
        .iter_mut()
        // Backwards
        .rev()
        // Ignore normalized elements
        .filter(|f| !f.is_zero_vartime())
        // Backwards, skip last element, fill in one for last term.
        .zip(prod.into_iter().rev().skip(1).chain(Some(Scalar::ONE)))
    {
        // tmp := tmp * f; f := tmp * s = 1/f
        let new_tmp = tmp * *f;
        *f = tmp * &s;
        tmp = new_tmp;
    }
}

#[cfg(test)]
mod tests {
    use ff::Field;
    use crate::Scalar;
    use super::{batch_inverse, batch_inverse_check_for_zero};

    fn random_elements(num_elements : usize) -> Vec<Scalar> {
        (0..num_elements)
        .map(|_| Scalar::random(&mut rand::thread_rng()))
        .collect::<Vec<_>>()
    }

    #[test]
    fn batch_inversion_smoke_test() {
        let random_elements = random_elements(1000);
        // A zero element is unlikely to be generated, however we check for it and swap it with 1, if thats the case
        let mut random_non_zero_elements = random_elements.into_iter().map(|f| if f.is_zero_vartime() { Scalar::ONE } else { f }).collect::<Vec<_>>();
        
        let got_inversion = random_non_zero_elements.iter().map(|f| f.invert().expect("unexpected zero scalar")).collect::<Vec<_>>();
        batch_inverse(&mut random_non_zero_elements);

        assert_eq!(random_non_zero_elements, got_inversion);
    }

    #[test]
    fn batch_inverse_zero_check() {
        let mut zero_elements = vec![Scalar::ZERO; 1000];
        batch_inverse_check_for_zero(&mut zero_elements);
        
        assert_eq!(zero_elements, vec![Scalar::ZERO; 1000]);
    }

    #[should_panic]
    #[test]
    fn batch_inverse_panic_check() {
        // Calling batch_inverse on a vector with a zero element should panic
        // One should call `batch_inverse_check_for_zero`
        let mut zero_elements = vec![Scalar::ZERO; 1000];
        batch_inverse(&mut zero_elements);
    }
}