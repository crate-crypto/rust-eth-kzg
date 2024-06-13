#[derive(Debug)]
pub enum Error {
    Jni(jni::errors::Error),
    ProverError(eip7594::prover::ProverError),
    VerifierError(eip7594::verifier::VerifierError),
}

impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Error::Jni(err)
    }
}

impl From<eip7594::prover::ProverError> for Error {
    fn from(err: eip7594::prover::ProverError) -> Self {
        Error::ProverError(err)
    }
}
