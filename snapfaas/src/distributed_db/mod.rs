pub mod db_server;
pub mod db_client;

pub const CACHE_ADDRESS: &str = "127.0.0.1:5000";
// delay time of db server in ms
pub const RESPONSE_DELAY_TIME: u64 = 50;

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