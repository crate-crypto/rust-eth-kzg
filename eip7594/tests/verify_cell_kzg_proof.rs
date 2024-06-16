use common::collect_test_files;
use eip7594::verifier::VerifierError;
use serde_::TestVector;
use std::fs;

mod common;

mod serde_ {
    use crate::common::{bytes_from_hex, UnsafeBytes};

    use serde::Deserialize;

    #[derive(Deserialize)]
    struct YamlInput {
        commitment: String,
        cell_id: u64,
        cell: String,
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
        pub cell_id: u64,
        pub cell: UnsafeBytes,
        pub proof: UnsafeBytes,
        pub output: Option<bool>,
    }

    impl TestVector {
        pub fn from_str(yaml_data: &str) -> Self {
            let yaml_test_vector: YamlTestVector = serde_yaml::from_str(yaml_data).unwrap();
            TestVector::from(yaml_test_vector)
        }
    }

    impl From<YamlTestVector> for TestVector {
        fn from(yaml_test_vector: YamlTestVector) -> Self {
            let commitment = bytes_from_hex(&yaml_test_vector.input.commitment);
            let proof = bytes_from_hex(&yaml_test_vector.input.proof);
            let cell_id = yaml_test_vector.input.cell_id;
            let cell = bytes_from_hex(&yaml_test_vector.input.cell);

            let output = yaml_test_vector.output;

            TestVector {
                commitment,
                cell_id,
                cell,
                proof,
                output,
            }
        }
    }
}

const TEST_DIR: &str = "../consensus_test_vectors/verify_cell_kzg_proof";
#[test]
fn test_verify_cell_kzg_proof() {
    let test_files = collect_test_files(TEST_DIR).unwrap();

    let verifier_context = eip7594::verifier::VerifierContext::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(&test_file).unwrap();
        let test = TestVector::from_str(&yaml_data);

        let cell = match (test.cell).try_into() {
            Ok(cell) => cell,
            Err(_) => {
                assert!(test.output.is_none());
                continue;
            }
        };
        let commitment = match (test.commitment).try_into() {
            Ok(commitment) => commitment,
            Err(_) => {
                assert!(test.output.is_none());
                continue;
            }
        };
        let proof = match (test.proof).try_into() {
            Ok(proof) => proof,
            Err(_) => {
                assert!(test.output.is_none());
                continue;
            }
        };

        match verifier_context.verify_cell_kzg_proof(&commitment, test.cell_id, &cell, &proof) {
            Ok(_) => {
                // We arrive at this point if the proof verified as true
                assert!(test.output.unwrap())
            }
            Err(VerifierError::InvalidProof) => {
                assert!(test.output.unwrap() == false);
            }
            Err(_) => {
                assert!(test.output.is_none());
            }
        };
    }
}
