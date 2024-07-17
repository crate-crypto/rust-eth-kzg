use common::collect_test_files;
use serde_::TestVector;
use std::fs;

mod common;

mod serde_ {
    use crate::common::{bytes_from_hex, UnsafeBytes};

    use serde::Deserialize;

    #[derive(Deserialize)]
    struct YamlInput {
        row_commitments: Vec<String>,
        row_indices: Vec<u64>,
        column_indices: Vec<u64>,
        cells: Vec<String>,
        proofs: Vec<String>,
    }

    type YamlOutput = bool;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub row_commitments: Vec<UnsafeBytes>,
        pub row_indices: Vec<u64>,
        pub column_indices: Vec<u64>,
        pub cells: Vec<UnsafeBytes>,
        pub proofs: Vec<UnsafeBytes>,
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
            let row_commitments = yaml_test_vector
                .input
                .row_commitments
                .into_iter()
                .map(|commitment| bytes_from_hex(&commitment))
                .collect();
            let cells = yaml_test_vector
                .input
                .cells
                .into_iter()
                .map(|cell| bytes_from_hex(&cell))
                .collect();
            let proofs: Vec<_> = yaml_test_vector
                .input
                .proofs
                .into_iter()
                .map(|proof| bytes_from_hex(&proof))
                .collect();
            let row_indices = yaml_test_vector.input.row_indices;
            let column_indices = yaml_test_vector.input.column_indices;

            let output = yaml_test_vector.output;

            TestVector {
                row_commitments,
                row_indices,
                column_indices,
                cells,
                proofs,
                output,
            }
        }
    }
}

const TEST_DIR: &str = "../consensus_test_vectors/verify_cell_kzg_proof_batch";
#[test]
fn test_verify_cell_kzg_proof_batch() {
    let test_files = collect_test_files(TEST_DIR).unwrap();

    let ctx = eip7594::PeerDASContext::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(&test_file).unwrap();
        let test = TestVector::from_str(&yaml_data);

        let cells: Result<_, _> = test
            .cells
            .iter()
            .map(Vec::as_slice)
            .map(|v| v.try_into())
            .collect();

        let cells = match cells {
            Ok(cells) => cells,
            Err(_) => {
                assert!(test.output.is_none());
                continue;
            }
        };

        let commitments: Result<_, _> = test
            .row_commitments
            .iter()
            .map(Vec::as_slice)
            .map(|v| v.try_into())
            .collect();

        let commitments = match commitments {
            Ok(commitments) => commitments,
            Err(_) => {
                assert!(test.output.is_none());
                continue;
            }
        };

        let proofs: Result<_, _> = test
            .proofs
            .iter()
            .map(Vec::as_slice)
            .map(|v| v.try_into())
            .collect();

        let proofs = match proofs {
            Ok(proofs) => proofs,
            Err(_) => {
                assert!(test.output.is_none());
                continue;
            }
        };

        match ctx.verify_cell_kzg_proof_batch(
            commitments,
            test.row_indices,
            test.column_indices,
            cells,
            proofs,
        ) {
            Ok(_) => {
                // We arrive at this point if the proof verified as true
                assert!(test.output.unwrap())
            }
            Err(x) if x.invalid_proof() => {
                assert!(test.output.unwrap() == false);
            }
            Err(_) => {
                assert!(test.output.is_none());
            }
        };
    }
}
