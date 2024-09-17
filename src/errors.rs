use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Error {
    #[cfg(feature = "client")]
    Credentials(CredentialsError),
    Transport(String)
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "client")]
            Error::Credentials(e) => write!(f, "Credentials error: {:?}", e),
            Error::Transport(e) => write!(f, "Transport error: {}", e)
        }
    }
}

#[cfg(feature = "client")]
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CredentialsError {}

#[cfg(feature = "tonic")]
impl From<tonic::transport::Error> for Error {
    fn from(e: tonic::transport::Error) -> Self {
        Error::Transport(e.to_string())
    }
}

#[cfg(feature = "tonic")]
impl From<tonic::Status> for Error {
    fn from(e: tonic::Status) -> Self {
        Error::Transport(format!("Status: {:?}", e))
    }
}
