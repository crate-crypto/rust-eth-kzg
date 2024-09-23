use blstrs::{G1Affine, G1Projective, Scalar};
use ff::Field;
use group::Group;

use crate::{
    batch_add::{batch_addition, batch_addition_diff_stride, multi_batch_addition_diff_stride},
    wnaf::wnaf_form,
};

// This just generalizes the double and add algorithm
pub struct SimpleMsm;

pub fn msm_sjf(points: &[G1Affine], scalars: &[Scalar]) -> G1Projective {
    let mut scalars_bytes: Vec<_> = scalars
        .into_iter()
        .map(|scalar| scalar.to_bytes_le())
        .collect();
    let scalars_jsf = calculate_dsjsf(&scalars_bytes);
    let mut buckets = vec![vec![]; 256];
    for (scalar_index, scalar_bits) in scalars_jsf.into_iter().enumerate() {
        for (index, bit) in scalar_bits.into_iter().enumerate() {
            if bit < 0 {
                buckets[index].push(-points[scalar_index]);
            } else if bit > 0 {
                buckets[index].push(points[scalar_index]);
            }
        }
    }

    let mut result = G1Projective::identity();
    let summed_windows = multi_batch_addition_diff_stride(buckets);
    for (window) in summed_windows.into_iter().rev() {
        result = result.double();
        result += window;
    }

    result
}
pub fn msm(points: &[G1Affine], scalars: &[Scalar]) -> G1Projective {
    let scalars_bits: Vec<_> = scalars.into_iter().map(|s| scalar_to_bits(*s)).collect();

    let mut buckets = vec![vec![]; 256];

    for (scalar_index, scalar_bits) in scalars_bits.into_iter().enumerate() {
        // iterate over scalar
        for (index, bit) in scalar_bits.into_iter().enumerate() {
            if bit != 0 {
                buckets[index].push(points[scalar_index]);
            }
        }
    }

    let mut result = G1Projective::identity();
    let summed_windows = multi_batch_addition_diff_stride(buckets);
    for (window) in summed_windows.into_iter().rev() {
        result = result.double();
        result += window;
    }

    result
}

pub fn scalar_to_bits(s: Scalar) -> [u8; 256] {
    let scalar_bytes = s.to_bytes_le();
    bytes_to_bits(scalar_bytes)
}
fn bytes_to_bits(bytes: [u8; 32]) -> [u8; 256] {
    let mut bit_vector = Vec::with_capacity(256);
    for byte in bytes {
        for i in 0..8 {
            bit_vector.push(((byte >> i) & 0x01) as u8)
        }
    }
    bit_vector.try_into().unwrap()
}

pub fn calculate_dsjsf(x: &[[u8; 32]]) -> Vec<Vec<i8>> {
    let d = x.len();
    let max_len = x.iter().map(|xi| xi.len()).max().unwrap_or(0);
    let mut result = vec![vec![0i8; 0]; d];
    let mut x_copy: Vec<Vec<u8>> = x.iter().map(|&xi| xi.to_vec()).collect();

    let mut j = 0;
    let mut a = vec![Vec::new(); 2];

    loop {
        if x_copy.iter().all(|xi| xi.iter().all(|&b| b == 0)) {
            break;
        }

        let mut xj = vec![0i8; d];
        a.push(Vec::new());

        for k in 0..d {
            if let Some(&last_byte) = x_copy[k].last() {
                xj[k] = (last_byte & 1) as i8;
                if xj[k] == 1 {
                    a[j].push(k);
                }
            }
        }

        for k in 0..d {
            if x_copy[k].len() > 1 || (x_copy[k].len() == 1 && x_copy[k][0] > 1) {
                let next_bit = ((x_copy[k].last().unwrap_or(&0) >> 1) & 1) as i8;
                if next_bit == 1 {
                    a[j + 1].push(k);
                }
            }
        }

        if a[j + 1].iter().all(|&k| a[j].contains(&k)) {
            for &k in &a[j + 1] {
                xj[k] = -xj[k];
            }
            a[j + 1].clear();
        } else {
            for &k in &a[j] {
                if !a[j + 1].contains(&k) {
                    xj[k] = -xj[k];
                }
            }
            a[j + 1] = a[j]
                .iter()
                .cloned()
                .chain(a[j + 1].iter().cloned())
                .collect();
        }

        for k in 0..d {
            result[k].insert(0, xj[k]);
            if !x_copy[k].is_empty() {
                let mut borrow = (xj[k] < 0) as u8;
                for byte in x_copy[k].iter_mut().rev() {
                    let (new_byte, new_borrow) = byte.overflowing_sub(borrow);
                    *byte = new_byte;
                    if new_borrow {
                        borrow = 1;
                    } else {
                        break;
                    }
                }
                divide_by_two(&mut x_copy[k]);
            }
        }

        j += 1;
    }

    result
}

/// Helper function to divide a big-endian byte array by 2
fn divide_by_two(num: &mut Vec<u8>) {
    let mut carry = 0;
    for byte in num.iter_mut().rev() {
        let new_carry = *byte & 1;
        *byte = (*byte >> 1) | (carry << 7);
        carry = new_carry;
    }
    while num.len() > 1 && num[0] == 0 {
        num.remove(0);
    }
}

fn random_points(num_points: usize) -> Vec<G1Affine> {
    use group::Group;
    (0..num_points)
        .into_iter()
        .map(|_| G1Projective::random(&mut rand::thread_rng()).into())
        .collect()
}

#[test]
fn test_simple_msm() {
    use ff::Field;
    let num_points = 64;
    let points = random_points(num_points);
    let scalars: Vec<_> = (0..num_points)
        .into_iter()
        .map(|i| Scalar::random(&mut rand::thread_rng()))
        .collect();
    let now = std::time::Instant::now();
    msm_sjf(&points, &scalars);
    dbg!(now.elapsed().as_micros());
}
