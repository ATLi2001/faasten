pub mod db_server;
pub mod db_client;

#[derive(Debug)]
pub enum Error {
    TcpConnectionError,
    TcpIOError(std::io::Error),
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::TcpIOError(e)
    }
}

pub trait DbService {
    // get value at key
    fn get(&self, key: Vec<u8>) -> Result<Vec<u8>, Error>;
    // put value at key, no write flags
    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error>;
    // put value at key, no overwrite flags
    fn add(&self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, Error>;
    // compare and swap
    fn cas(&self, key: Vec<u8>, expected: Option<Vec<u8>>, value: Vec<u8>) -> Result<Vec<u8>, Error>;
    // scan directory
    fn scan(&self, dir: Vec<u8>) -> Result<Vec<u8>, Error>;
}