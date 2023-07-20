#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Error {
    #[cfg(feature = "client")]
    Credentials(CredentialsError),
}

#[cfg(feature = "client")]
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum CredentialsError {}