pub mod db_server;
pub mod db_client;

pub trait DbService {
    fn get(&mut self, key: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>, flags: Option<u32>) -> Result<Vec<u8>, db_client::Error>;
    fn scan(&mut self, dir: Vec<u8>) -> Result<Vec<u8>, db_client::Error>;
}