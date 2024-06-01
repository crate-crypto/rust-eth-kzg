use common::collect_test_files;
use serde_::TestVector;
use std::fs;

mod common;

mod serde_ {
    use crate::common::bytes48_from_hex;

    use super::common::bytes_from_hex;
    use eip7594::{Blob, Bytes48};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct YamlInput {
        blob: String,
    }

    type YamlOutput = String;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub blob: Blob,
        pub commitment: Option<Bytes48>,
    }

    impl TestVector {
        pub fn from_str(yaml_data: &str) -> Self {
            let yaml_test_vector: YamlTestVector = serde_yaml::from_str(yaml_data).unwrap();
            TestVector::from(yaml_test_vector)
        }
    }

    impl From<YamlTestVector> for TestVector {
        fn from(yaml_test_vector: YamlTestVector) -> Self {
            let input = yaml_test_vector.input.blob;
            let output = yaml_test_vector.output;

            let input = bytes_from_hex(&input);

            let commitment = match output {
                Some(commitment) => Some(bytes48_from_hex(&commitment)),
                None => None,
            };

            TestVector {
                blob: input,
                commitment,
            }
        }
    }
}

const TEST_DIR: &str = "../consensus_test_vectors/blob_to_kzg_commitment";
#[test]
fn test_blob_to_kzg_commitment() {
    let test_files = collect_test_files(TEST_DIR).unwrap();

    let prover_context = eip7594::prover::ProverContext::new();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(test_file).unwrap();
        let test = TestVector::from_str(&yaml_data);

        match prover_context.blob_to_kzg_commitment(&test.blob) {
            Ok(commitment) => {
                let expected_commitment = test.commitment.unwrap();

                assert_eq!(commitment, expected_commitment);
            }
            Err(_) => {
                // On an error, we expect the output to be null
                assert!(test.commitment.is_none());
            }
        };
    }
}
