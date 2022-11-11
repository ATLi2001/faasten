pub mod db_server;
pub mod db_client;

pub trait DbService {
    // get value at key
    fn get(&mut self, key: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
    // put value at key, no write flags
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
    // put value at key, no overwrite flags
    fn add(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
    // compare and swap
    fn cas(&mut self, key: Vec<u8>, expected: Option<Vec<u8>>, value: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
    // scan directory
    fn scan(&mut self, dir: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
}