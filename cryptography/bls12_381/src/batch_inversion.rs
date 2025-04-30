use ff::Field;

/// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
///
/// Panics if any of the elements are zero
pub fn batch_inverse<F: Field>(v: &mut [F]) {
    let mut scratch_pad = Vec::with_capacity(v.len());
    batch_inverse_scratch_pad(v, &mut scratch_pad);
}

/// Given a vector of field elements {v_i}, compute the vector {v_i^(-1)}
///
/// A scratchpad is used to avoid excessive allocations in the case that this method is
/// called repeatedly.
///
/// Panics if any of the elements are zero
pub fn batch_inverse_scratch_pad<F: Field>(v: &mut [F], scratchpad: &mut Vec<F>) {
    // Montgomery's Trick and Fast Implementation of Masked AES
    // Genelle, Prouff and Quisquater
    // Section 3.2
    // but with an optimization to multiply every element in the returned vector by coeff

    let n = v.len();
    if n == 0 {
        return;
    }

    // Clear the scratchpad and ensure it has enough capacity
    scratchpad.clear();
    scratchpad.reserve(n);

    // First pass: compute [a, ab, abc, ...]
    let mut tmp = F::ONE;
    for f in v.iter() {
        tmp *= f;
        scratchpad.push(tmp);
    }

    // Invert `tmp`.
    tmp = tmp
        .invert()
        .expect("guaranteed to be non-zero since we filtered out zero field elements");

    // Second pass: iterate backwards to compute inverses
    for (f, s) in v
        .iter_mut()
        // Backwards
        .rev()
        // Backwards, skip last element, fill in one for last term.
        .zip(scratchpad.iter().rev().skip(1).chain(Some(&F::ONE)))
    {
        // tmp := tmp * f; f := tmp * s = 1/f
        let new_tmp = tmp * *f;
        *f = tmp * *s;
        tmp = new_tmp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blstrs::Scalar;

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

    #[should_panic]
    #[test]
    fn batch_inverse_panic_check() {
        // Calling batch_inverse on a vector with a zero element should panic
        let mut zero_elements = vec![Scalar::ZERO; 1000];
        batch_inverse(&mut zero_elements);
    }
}
