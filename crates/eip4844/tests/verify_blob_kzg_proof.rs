use std::fs;

use common::collect_test_files;
use serde_::TestVector;
use tbd::{
    constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT},
    Error, VerifierError,
};

#[path = "../../eip7594/tests/common.rs"]
mod common;

mod serde_ {

    use serde::Deserialize;

    use super::common::{bytes_from_hex, UnsafeBytes};

    #[derive(Deserialize)]
    struct YamlInput {
        blob: String,
        commitment: String,
        proof: String,
    }

    type YamlOutput = bool;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub blob: UnsafeBytes,
        pub commitment: UnsafeBytes,
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
            let blob = yaml_test_vector.input.blob;
            let commitment = yaml_test_vector.input.commitment;
            let proof = yaml_test_vector.input.proof;
            let output = yaml_test_vector.output;

            let blob = bytes_from_hex(&blob);
            let commitment = bytes_from_hex(&commitment);
            let proof = bytes_from_hex(&proof);

            Self {
                blob,
                commitment,
                proof,
                output,
            }
        }
    }
}

const TEST_DIR: &str = "../../test_vectors/verify_blob_kzg_proof";
#[test]
fn test_verify_blob_kzg_proof() {
    let test_files = collect_test_files(TEST_DIR).expect("unable to collect test files");

    let ctx = tbd::Context::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(test_file).expect("unable to read test file");
        let test = TestVector::from_str(&yaml_data);

        let blob: &[u8; BYTES_PER_BLOB] = if let Ok(blob) = (&test.blob[..]).try_into() {
            blob
        } else {
            // Blob does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        let commitment: [u8; BYTES_PER_COMMITMENT] =
            if let Ok(commitment) = test.commitment.try_into() {
                commitment
            } else {
                // Commitment does not have a valid size
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

        match ctx.verify_blob_kzg_proof(blob, commitment, proof) {
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
