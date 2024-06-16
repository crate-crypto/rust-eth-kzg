use common::collect_test_files;
use serde_::TestVector;
use std::fs;

mod common;

mod serde_ {
    use crate::common::{bytes_from_hex, UnsafeBytes};

    use serde::Deserialize;

    #[derive(Deserialize)]
    struct YamlInput {
        cell_ids: Vec<u64>,
        cells: Vec<String>,
    }

    type YamlOutput = Vec<String>;

    #[derive(Deserialize)]
    struct YamlTestVector {
        input: YamlInput,
        output: Option<YamlOutput>,
    }

    pub struct TestVector {
        pub input_cell_ids: Vec<u64>,
        pub input_cells: Vec<UnsafeBytes>,
        pub output_cells: Option<Vec<UnsafeBytes>>,
    }

    impl TestVector {
        pub fn from_str(yaml_data: &str) -> Self {
            let yaml_test_vector: YamlTestVector = serde_yaml::from_str(yaml_data).unwrap();
            TestVector::from(yaml_test_vector)
        }
    }

    impl From<YamlTestVector> for TestVector {
        fn from(yaml_test_vector: YamlTestVector) -> Self {
            let cell_ids = yaml_test_vector.input.cell_ids;

            let input_cells: Vec<_> = yaml_test_vector
                .input
                .cells
                .iter()
                .map(|cell| bytes_from_hex(cell))
                .collect();

            let output = match yaml_test_vector.output {
                Some(cells) => {
                    let cells: Vec<_> = cells
                        .into_iter()
                        .map(|cell| bytes_from_hex(&cell))
                        .collect();
                    Some(cells)
                }
                None => None,
            };

            TestVector {
                input_cell_ids: cell_ids,
                input_cells: input_cells,
                output_cells: output,
            }
        }
    }
}

const TEST_DIR: &str = "../consensus_test_vectors/recover_all_cells";
#[test]
fn test_recover_all_cells() {
    let test_files = collect_test_files(TEST_DIR).unwrap();

    let verifier_context = eip7594::verifier::VerifierContext::default();

    for test_file in test_files {
        let yaml_data = fs::read_to_string(test_file).unwrap();
        let test = TestVector::from_str(&yaml_data);

        let input_cells: Result<_, _> = test
            .input_cells
            .iter()
            .map(Vec::as_slice)
            .map(|v| v.try_into())
            .collect();

        let input_cells = match input_cells {
            Ok(input_cells) => input_cells,
            Err(_) => {
                assert!(test.output_cells.is_none());
                continue;
            }
        };

        match verifier_context.recover_all_cells(test.input_cell_ids, input_cells) {
            Ok(cells) => {
                let expected_cells = test.output_cells.unwrap();

                for k in 0..expected_cells.len() {
                    let expected_cell = &expected_cells[k];

                    let got_cell = &cells[k];

                    assert_eq!(&got_cell[..], expected_cell);
                }
            }
            Err(_) => {
                // On an error, we expect the output to be null
                assert!(test.output_cells.is_none());
            }
        };
    }
}
