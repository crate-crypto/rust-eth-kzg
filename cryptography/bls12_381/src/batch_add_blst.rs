use blstrs::{Fp, G1Affine, G1Projective};
use ff::Field;
use subtle::{Choice, ConditionallySelectable};

#[derive(Debug, Clone, Copy)]
pub struct G1AffineInv {
    pub x: Fp,
    pub y: Fp,
    pub tmp: Fp,
}

impl From<G1Affine> for G1AffineInv {
    fn from(value: G1Affine) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
            tmp: Fp::ZERO,
        }
    }
}

// fn is_zero(point: &G1AffineInv) -> Choice {
//     point.x.is_zero() & point.y.is_zero()
// }

// /*

// COPIED FROM BLST!!!!!!!!!!

//  * This implementation uses explicit addition formula:
//  *
//  * λ = (Y₂-Y₁)/(X₂-X₁)
//  * X₃ = λ²-(X₁+X₂)
//  * Y₃ = λ⋅(X₁-X₃)-Y₁
//  *
//  * But since we don't know if we'll have to add point to itself, we need
//  * to eventually resort to corresponding doubling formula:
//  *
//  * λ = 3X₁²/2Y₁
//  * X₃ = λ²-2X₁
//  * Y₃ = λ⋅(X₁-X₃)-Y₁
//  *
//  * The formulae use prohibitively expensive inversion, but whenever we
//  * have a lot of affine points to accumulate, we can amortize the cost
//  * by applying Montgomery's batch inversion approach. As a result,
//  * asymptotic[!] per-point cost for addition is as small as 5M+1S. For
//  * comparison, ptype##_dadd_affine takes 8M+5S. In practice, all things
//  * considered, the improvement coefficient varies from 60% to 85%
//  * depending on platform and curve.
//  *
//  * THIS IMPLEMENTATION IS *NOT* CONSTANT-TIME. [But if there is an
//  * application that requires constant time-ness, speak up!]
//  */
// /*
//  * Calculate λ's numerator and denominator.
//  *
//  * input:	A	x1	y1	-
//  *		B	x2	y2	-
//  * output:
//  * if A!=B:	A	x1	y1	(x2-x1)*mul_acc
//  *		B	x2+x1	y2-y1	(x2-x1)
//  *
//  * if A==B:	A	x	y	2y*mul_acc
//  *		B	2x	3*x^2	2y
//  *
//  * if A==-B:	A	0	0	1*mul_acc
//  *		B	0	3*x^2	0
//  */
// fn head(a: &mut G1AffineInv, b: &mut G1AffineInv, mul_acc: Option<&Fp>) {
//     let inf = is_zero(a) | is_zero(b);
//     let zero = Fp::ZERO;
//     let one = Fp::ONE;

//     // X2-X1
//     b.tmp = b.x - a.x;

//     // X2+X1
//     let x_sum = b.x + a.x;

//     // Y2+Y1
//     let y_sum = b.y + a.y;

//     // Y2-Y1
//     let y_diff = b.y - a.y;

//     if b.tmp.is_zero().into() {
//         // X2==X1
//         let inf_inner = a.tmp.is_zero();
//         b.x = Fp::conditional_select(&b.x, &a.tmp, inf_inner);

//         // 3*X1^2
//         b.y = a.x.square();
//         b.y *= Fp::from(3u64);

//         // 2*Y1
//         b.tmp = a.tmp;
//     }

//     // Conditional selections
//     a.x = Fp::conditional_select(&a.x, &b.x, inf);
//     a.y = Fp::conditional_select(&a.y, &a.tmp, inf);
//     a.tmp = Fp::conditional_select(&one, &b.tmp, inf);
//     b.tmp = Fp::conditional_select(&zero, &b.tmp, inf);

//     // Chain multiplication
//     if let Some(acc) = mul_acc {
//         a.tmp *= acc;
//     }

//     // Update b
//     b.x = x_sum;
//     b.y = y_diff;
// }

// fn tail(d: &mut G1AffineInv, ab: &[G1AffineInv; 2], mut lambda: Fp) {
//     let a = &ab[0];
//     let b = &ab[1];

//     let inf = b.tmp.is_zero();
//     let one = Fp::ONE;

//     // λ = (Y2-Y1)/(X2-X1) or 3*X1^2/2*Y1
//     lambda *= b.y;

//     // llambda = λ^2
//     let llambda = lambda.square();

//     // X3 = λ^2 - (X2+X1)
//     d.x = llambda - b.x;

//     // Y3 = λ*(X1-X3) - Y1
//     d.y = a.x - d.x;
//     d.y *= lambda;
//     d.y -= a.y;

//     // Conditional selection for point at infinity
//     d.x = Fp::conditional_select(&d.x, &a.x, inf);
//     d.y = Fp::conditional_select(&d.y, &a.y, inf);

//     // This seems to be
//     // setting B->Z to 1 if the result is the point at infinity
//     let mut b_tmp = Fp::conditional_select(&b.tmp, &one, inf);
// }

// fn dadd_affine(sum: &mut G1Projective, point: &G1Affine) {
//     sum.add_assign_mixed(point);
// }

// fn accumulate(sum: &mut G1Projective, mut points: &mut [G1AffineInv]) {
//     let mut n = points.len();

//     while n >= 16 {
//         if n & 1 != 0 {
//             let affine_point = G1Affine::from_raw_unchecked(points[0].x, points[0].y, false);
//             dadd_affine(sum, &affine_point);
//             points = &mut points[1..];
//             n -= 1;
//         }
//         n /= 2;

//         let mut mul_acc = None;
//         for i in 0..n {
//             head(&mut points[2 * i], &mut points[2 * i + 1], mul_acc);
//             mul_acc = Some(&points[2 * i].tmp);
//         }

//         points[2 * n - 2].tmp = points[2 * n - 2].tmp.invert().unwrap();

//         let mut dst = n;
//         for i in (1..n).rev() {
//             dst -= 1;
//             points[2 * i - 2].tmp = points[2 * i - 2].tmp * points[2 * i].tmp;
//             tail(
//                 &mut points[dst],
//                 &[points[2 * i - 2], points[2 * i - 1]],
//                 points[2 * i - 2].tmp,
//             );
//             points[2 * i - 2].tmp = points[2 * i - 2].tmp * points[2 * i + 1].tmp;
//         }
//         dst -= 1;
//         tail(&mut points[dst], &[points[0], points[1]], points[0].tmp);
//         points = &mut points[..n];
//     }

//     for point in points.iter() {
//         let affine_point = G1Affine::from_raw_unchecked(point.x, point.y, false);
//         dadd_affine(sum, &affine_point);
//     }
// }
