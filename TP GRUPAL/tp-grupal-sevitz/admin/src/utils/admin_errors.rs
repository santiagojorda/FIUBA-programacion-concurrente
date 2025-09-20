#[derive(Debug, PartialEq)]
pub enum AdminError {
    /// IO error
    IoError(String),
    BindError(String),
    InvalidPeers(String),
}

impl From<std::io::Error> for AdminError {
    /// std::io::Error to AdminError
    fn from(err: std::io::Error) -> AdminError {
        AdminError::IoError(err.to_string())
    }
}
