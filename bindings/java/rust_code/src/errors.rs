#[derive(Debug)]
pub enum Error {
    Jni(jni::errors::Error),
    IncorrectSize {
        expected: usize,
        got: usize,
        name: &'static str,
    },
    Prover(rust_eth_kzg::prover::ProverError),
    Verifier(rust_eth_kzg::verifier::VerifierError),
}

impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Error::Jni(err)
    }
}

impl From<rust_eth_kzg::prover::ProverError> for Error {
    fn from(err: rust_eth_kzg::prover::ProverError) -> Self {
        Error::Prover(err)
    }
}
