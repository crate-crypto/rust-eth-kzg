use blstrs::{G1Affine, G1Projective};
use group::{WnafBase, WnafScalar};

use crate::Scalar;
// pub fn foo(wnaf_bases : &Vec<WnafBase<G1Projective, 8>>, scalars : Vec<Scalar>) {

// // let wnaf_bases: Vec<_> = bases.into_iter().map(WnafBase::<_, 8>::new).collect();
// let wnaf_scalars: Vec<_> = scalars.iter().map(WnafScalar::new).collect();
// let results: Vec<_> = wnaf_bases
//     .into_iter()
//     .flat_map(|base| wnaf_scalars.iter().map(|scalar| base * scalar))
//     .collect();
// }
#[test]
fn okay() {
    use ff::Field;
    use group::Group;
    let bases: Vec<G1Projective> = (0..4096).map(|_| G1Projective::generator()).collect();
    let scalars: Vec<_> = (0..4096)
        .map(|_| Scalar::random(&mut rand::thread_rng()))
        .collect();

    let now = std::time::Instant::now();
    for (scalar, base) in scalars.iter().zip(bases.iter()) {
        let _ = base * scalar;
    }
    println!("Time normal: {:?}", now.elapsed());

    let wnaf_bases: Vec<_> = bases.into_iter().map(WnafBase::<_, 8>::new).collect();

    let now = std::time::Instant::now();
    let wnaf_scalars: Vec<_> = scalars.iter().map(WnafScalar::new).collect();
    for (wnaf_base, wnaf_scalar) in wnaf_bases.iter().zip(wnaf_scalars.iter()) {
        let _ = wnaf_base * wnaf_scalar;
    }
    // let results: Vec<_> = wnaf_bases
    //     .into_iter()
    //     .flat_map(|base| wnaf_scalars.iter().map(|scalar| base * scalar))
    //     .collect();
    println!("Time: {:?}", now.elapsed());

    let mut foo = G1Projective::generator() * Scalar::random(&mut rand::thread_rng());
    let mut result = G1Projective::generator() * Scalar::random(&mut rand::thread_rng());
    let now: std::time::Instant = std::time::Instant::now();
    for _ in 0..256 / 8 {
        result += foo;
        // for j in 0..8 {
        //     foo = foo.double()
        // }
    }
    println!("Time for 256 additions: {:?}", now.elapsed());
}

// pub fn multi_exp(points: &[Self], scalars: &[Scalar]) -> Self {
//     let n = if points.len() < scalars.len() {
//         points.len()
//     } else {
//         scalars.len()
//     };
//     let points =
//         unsafe { std::slice::from_raw_parts(points.as_ptr() as *const blst_p1, points.len()) };

//     let points = p1_affines::from(points);

//     let mut scalar_bytes: Vec<u8> = Vec::with_capacity(n * 32);
//     for a in scalars.iter().map(|s| s.to_bytes_le()) {
//         scalar_bytes.extend_from_slice(&a);
//     }

//     let res = points.mult(scalar_bytes.as_slice(), 255);

//     G1Projective(res)
// }
