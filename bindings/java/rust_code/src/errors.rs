use eip7594::Error as DASError;

#[derive(Debug)]
pub enum Error {
    Jni(jni::errors::Error),
    IncorrectSize {
        expected: usize,
        got: usize,
        name: &'static str,
    },
    DASError(DASError),
}

impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Error::Jni(err)
    }
}

impl From<DASError> for Error {
    fn from(err: DASError) -> Self {
        Error::DASError(err)
    }
}
