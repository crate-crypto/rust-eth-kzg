use std::fs;

use common::collect_test_files;
use serde_::TestVector;
use tbd::{constants::BYTES_PER_BLOB, Error, KZGCommitment, KZGProof, VerifierError};

#[path = "../../eip7594/tests/common.rs"]
mod common;

mod serde_ {

    use serde::Deserialize;

    use super::common::{bytes_from_hex, UnsafeBytes};

    #[derive(Deserialize)]
    struct YamlInput {
        blobs: Vec<String>,
        commitments: Vec<String>,
        proofs: Vec<String>,
    }

    type YamlOutput = bool;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub blobs: Vec<UnsafeBytes>,
        pub commitments: Vec<UnsafeBytes>,
        pub proofs: Vec<UnsafeBytes>,
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
            let blobs = yaml_test_vector.input.blobs;
            let commitments = yaml_test_vector.input.commitments;
            let proofs = yaml_test_vector.input.proofs;
            let output = yaml_test_vector.output;

            let blobs = blobs.iter().map(|blob| bytes_from_hex(blob)).collect();
            let commitments = commitments
                .iter()
                .map(|commitment| bytes_from_hex(commitment))
                .collect();
            let proofs = proofs.iter().map(|proof| bytes_from_hex(proof)).collect();

            Self {
                blobs,
                commitments,
                proofs,
                output,
            }
        }
    }
}

const TEST_DIR: &str = "../../test_vectors/verify_blob_kzg_proof_batch";
#[test]
fn test_verify_blob_kzg_proof_batch() {
    let test_files = collect_test_files(TEST_DIR).expect("unable to collect test files");

    let ctx = tbd::Context::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(test_file).expect("unable to read test file");
        let test = TestVector::from_str(&yaml_data);

        let Ok(blobs) = test
            .blobs
            .iter()
            .map(|blob| <&[u8; BYTES_PER_BLOB]>::try_from(blob.as_slice()))
            .collect::<Result<Vec<_>, _>>()
        else {
            // Blob does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        let Ok(commitments) = test
            .commitments
            .iter()
            .map(|commitment| KZGCommitment::try_from(commitment.as_slice()))
            .collect::<Result<Vec<_>, _>>()
        else {
            // Commitment does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        let Ok(proofs) = test
            .proofs
            .iter()
            .map(|proof| KZGProof::try_from(proof.as_slice()))
            .collect::<Result<Vec<_>, _>>()
        else {
            // Proof does not have a valid size
            assert!(test.output.is_none());
            continue;
        };

        match ctx.verify_blob_kzg_proof_batch(&blobs, &commitments, &proofs) {
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
