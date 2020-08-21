use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid I/O")]
    Io(#[from] io::Error),
}

impl From<ConfigError> for io::Error {
    fn from(err: ConfigError) -> io::Error {
        match err {
            _ => io::Error::new(io::ErrorKind::Other, format!("{}", err)),
        }
    }
}
