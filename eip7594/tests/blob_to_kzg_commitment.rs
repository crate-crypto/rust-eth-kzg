use common::collect_test_files;
use rust_eth_kzg::constants::BYTES_PER_BLOB;
use serde_::TestVector;
use std::fs;

mod common;

mod serde_ {

    use crate::common::UnsafeBytes;

    use super::common::bytes_from_hex;
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
        pub blob: UnsafeBytes,
        pub commitment: Option<UnsafeBytes>,
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
                Some(commitment) => Some(bytes_from_hex(&commitment)),
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

    let ctx = rust_eth_kzg::PeerDASContext::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(test_file).unwrap();
        let test = TestVector::from_str(&yaml_data);

        //
        let blob: &[u8; BYTES_PER_BLOB] = match (&test.blob[..]).try_into() {
            Ok(blob) => blob,
            Err(_) => {
                // Blob does not have a valid size
                assert!(test.commitment.is_none());
                continue;
            }
        };

        match ctx.blob_to_kzg_commitment(blob) {
            Ok(commitment) => {
                let expected_commitment = test.commitment.unwrap();

                assert_eq!(&commitment[..], &expected_commitment);
            }
            Err(_) => {
                // On an error, we expect the output to be null
                assert!(test.commitment.is_none());
            }
        };
    }
}
