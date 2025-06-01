use std::fs;

use common::collect_test_files;
use eip4844::{
    constants::{BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT},
    Error, VerifierError,
};
use serde_::TestVector;

mod common;

mod serde_ {

    use serde::Deserialize;

    use super::common::{bytes_from_hex, UnsafeBytes};

    #[derive(Deserialize)]
    struct YamlInput {
        commitment: String,
        z: String,
        y: String,
        proof: String,
    }

    type YamlOutput = bool;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub commitment: UnsafeBytes,
        pub z: UnsafeBytes,
        pub y: UnsafeBytes,
        pub proof: UnsafeBytes,
        pub output: Option<bool>,
    }

    impl TestVector {
        pub fn from_str(yaml_data: &str) -> Self {
            let yaml_test_vector: YamlTestVector =
                serde_yaml::from_str(yaml_data).expect("invalid yaml");
            Self::from(yaml_test_vector)
        }
    }

    impl From<YamlTestVector> for TestVector {
        fn from(yaml_test_vector: YamlTestVector) -> Self {
            let commitment = yaml_test_vector.input.commitment;
            let z = yaml_test_vector.input.z;
            let y = yaml_test_vector.input.y;
            let proof = yaml_test_vector.input.proof;
            let output = yaml_test_vector.output;

            let commitment = bytes_from_hex(&commitment);
            let z = bytes_from_hex(&z);
            let y = bytes_from_hex(&y);
            let proof = bytes_from_hex(&proof);

            Self {
                commitment,
                z,
                y,
                proof,
                output,
            }
        }
    }
}

const TEST_DIR: &str = "../../test_vectors/verify_kzg_proof";
#[test]
fn test_verify_kzg_proof() {
    let test_files = collect_test_files(TEST_DIR).expect("unable to collect test files");

    let ctx = eip4844::Context::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(test_file).expect("unable to read test file");
        let test = TestVector::from_str(&yaml_data);

        let commitment: [u8; BYTES_PER_COMMITMENT] =
            if let Ok(commitment) = test.commitment.try_into() {
                commitment
            } else {
                // Commitment does not have a valid size
                assert!(test.output.is_none());
                continue;
            };

        let z: [u8; BYTES_PER_FIELD_ELEMENT] = if let Ok(z) = test.z.try_into() {
            z
        } else {
            // Point does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        let y: [u8; BYTES_PER_FIELD_ELEMENT] = if let Ok(y) = test.y.try_into() {
            y
        } else {
            // Evaluation does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        let proof: [u8; BYTES_PER_COMMITMENT] = if let Ok(proof) = test.proof.try_into() {
            proof
        } else {
            // Proof does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        match ctx.verify_kzg_proof(&commitment, z, y, &proof) {
            Ok(()) => {
                // We arrive at this point if the proof verified as true
                assert!(test.output.unwrap());
            }
            Err(Error::Verifier(VerifierError::InvalidProof)) => {
                assert!(!test.output.unwrap());
            }
            Err(_) => {
                assert!(test.output.is_none());
            }
        }
    }
}
