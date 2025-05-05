use std::fs;

use common::collect_test_files;
use serde_::TestVector;
use tbd::constants::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT};

#[path = "../../eip7594/tests/common.rs"]
mod common;

mod serde_ {

    use serde::Deserialize;

    use super::common::{bytes_from_hex, UnsafeBytes};

    #[derive(Deserialize)]
    struct YamlInput {
        blob: String,
        z: String,
    }

    type YamlOutput = [String; 2];

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub blob: UnsafeBytes,
        pub z: UnsafeBytes,
        pub output: Option<[UnsafeBytes; 2]>,
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
            let z = yaml_test_vector.input.z;
            let output = yaml_test_vector.output;

            let blob = bytes_from_hex(&blob);
            let z = bytes_from_hex(&z);

            let output = output.map(|output| output.map(|output| bytes_from_hex(&output)));

            Self { blob, z, output }
        }
    }
}

const TEST_DIR: &str = "../../test_vectors/compute_kzg_proof";
#[test]
fn test_compute_kzg_proof() {
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

        let z: [u8; BYTES_PER_FIELD_ELEMENT] = if let Ok(z) = test.z.try_into() {
            z
        } else {
            // z does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        match ctx.compute_kzg_proof(blob, z) {
            Ok((proof, evaluation)) => {
                let [expected_proof, expected_evaluation] = test.output.expect("output is none");

                assert_eq!(&proof[..], &expected_proof);
                assert_eq!(&evaluation[..], &expected_evaluation);
            }
            Err(_) => {
                // On an error, we expect the output to be null
                assert!(test.output.is_none());
            }
        }
    }
}
