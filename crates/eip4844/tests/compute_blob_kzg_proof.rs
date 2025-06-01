use std::fs;

use common::collect_test_files;
use eip4844::constants::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT};
use serde_::TestVector;

mod common;

mod serde_ {

    use serde::Deserialize;

    use super::common::{bytes_from_hex, UnsafeBytes};

    #[derive(Deserialize)]
    struct YamlInput {
        blob: String,
        commitment: String,
    }

    type YamlOutput = String;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub blob: UnsafeBytes,
        pub commitment: UnsafeBytes,
        pub output: Option<UnsafeBytes>,
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
            let output = yaml_test_vector.output;

            let blob = bytes_from_hex(&blob);
            let commitment = bytes_from_hex(&commitment);

            let output = output.map(|output| bytes_from_hex(&output));

            Self {
                blob,
                commitment,
                output,
            }
        }
    }
}

const TEST_DIR: &str = "../../test_vectors/compute_blob_kzg_proof";
#[test]
fn test_compute_blob_kzg_proof() {
    let test_files = collect_test_files(TEST_DIR).expect("unable to collect test files");

    let ctx = eip4844::Context::default();

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
                // z does not have a valid size
                assert!(test.output.is_none());
                continue;
            };

        match ctx.compute_blob_kzg_proof(blob, &commitment) {
            Ok(proof) => {
                let expected_proof = test.output.expect("output is none");

                assert_eq!(&proof[..], &expected_proof);
            }
            Err(_) => {
                // On an error, we expect the output to be null
                assert!(test.output.is_none());
            }
        }
    }
}
