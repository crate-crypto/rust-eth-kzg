use rayon::prelude::*;

/// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
//
/// Panics: If any of the elements are zero
pub fn batch_inverse<F: ff::Field>(elements: &mut [F]) {
    batch_inversion(elements)
}

/// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
///
// Taken and modified from arkworks codebase
fn batch_inversion<F: ff::Field>(v: &mut [F]) {
    // Divide the vector v evenly between all available cores
    let min_elements_per_thread = 1;
    let num_cpus_available = rayon::current_num_threads();
    let num_elems = v.len();
    let num_elem_per_thread =
        std::cmp::max(num_elems / num_cpus_available, min_elements_per_thread);

    // Batch invert in parallel, without copying the vector
    v.par_chunks_mut(num_elem_per_thread).for_each(|chunk| {
        serial_batch_inversion(chunk);
    });
}

/// Given a vector of field elements {v_i}, compute the vector {coeff * v_i^(-1)}
/// This method is explicitly single core.
fn serial_batch_inversion<F: ff::Field>(v: &mut [F]) {
    // Montgomery’s Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2
    // but with an optimization to multiply every element in the returned vector by coeff

    // First pass: compute [a, ab, abc, ...]
    let mut prod = Vec::with_capacity(v.len());
    let mut tmp = F::ONE;
    for f in v.iter().filter(|f| !f.is_zero_vartime()) {
        tmp.mul_assign(f);
        prod.push(tmp);
    }

    assert_eq!(prod.len(), v.len(), "inversion by zero is not allowed");

    // Invert `tmp`.
    tmp = tmp
        .invert()
        .expect("guaranteed to be non-zero since we filtered out zero field elements");

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v
        .iter_mut()
        // Backwards
        .rev()
        // Ignore normalized elements
        .filter(|f| !f.is_zero_vartime())
        // Backwards, skip last element, fill in one for last term.
        .zip(prod.into_iter().rev().skip(1).chain(Some(F::ONE)))
    {
        // tmp := tmp * f; f := tmp * s = 1/f
        let new_tmp = tmp * *f;
        *f = tmp * s;
        tmp = new_tmp;
    }
}

#[cfg(test)]
mod tests {
    use super::batch_inverse;
    use crate::Scalar;
    use ff::Field;

    fn random_elements(num_elements: usize) -> Vec<Scalar> {
        (0..num_elements)
            .map(|_| Scalar::random(&mut rand::thread_rng()))
            .collect::<Vec<_>>()
    }

    #[test]
    fn batch_inversion_smoke_test() {
        let random_elements = random_elements(1000);
        // A zero element is unlikely to be generated, however we check for it and swap it with 1, if thats the case
        let mut random_non_zero_elements = random_elements
            .into_iter()
            .map(|f| if f.is_zero_vartime() { Scalar::ONE } else { f })
            .collect::<Vec<_>>();

        let got_inversion = random_non_zero_elements
            .iter()
            .map(|f| f.invert().expect("unexpected zero scalar"))
            .collect::<Vec<_>>();
        batch_inverse(&mut random_non_zero_elements);

        assert_eq!(random_non_zero_elements, got_inversion);
    }

    // #[should_panic]
    // #[test]
    // fn batch_inverse_panic_check() {
    //     // Calling batch_inverse on a vector with a zero element should panic
    //     let mut zero_elements = vec![Scalar::ZERO; 1000];
    //     batch_inverse(&mut zero_elements);
    // }
}
