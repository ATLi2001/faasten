use std::net::{TcpListener, TcpStream};
use std::collections::HashSet;

use lmdb::Database;

use log::{error, debug};

use crate::syscalls;
use crate::labeled_fs::DBENV;


#[derive(Debug)]
pub enum Error {
    Rpc(prost::DecodeError),
    TcpWrite(std::io::Error),
    TcpRead(std::io::Error),
    RootfsNotExist,
}

#[derive(Debug)]
pub struct DbServer {
    address: String,
    db: Database,
}

impl DbServer {

    pub fn new(address: String) -> Self {
        let default_db = DBENV.open_db(None);
        if default_db.is_err() {
            error!("db error");
        }

        let default_db = default_db.unwrap();
        DbServer { address, db: default_db }
    }

    fn send_response(&self, mut stream: TcpStream, response: Vec<u8>) -> Result<(), Error> {
        use std::io::Write;

        stream.write_all(&(response.len() as u32).to_be_bytes()).map_err(|e| Error::TcpWrite(e))?;
        stream.write_all(response.as_ref()).map_err(|e| Error::TcpWrite(e))
    }

    fn handle_request(&self, stream: TcpStream) -> Result<(), Error> {
        use lmdb::{Transaction, WriteFlags};
        use prost::Message;
        use std::io::Read;
        use syscalls::syscall::Syscall as SC;
        use syscalls::Syscall;

        loop {
            let mut stream = stream.try_clone().unwrap();
            let mut lenbuf = [0;4];
            stream.read_exact(&mut lenbuf).map_err(|e| Error::TcpRead(e))?;
            let size = u32::from_be_bytes(lenbuf);
            let mut buf = vec![0u8; size as usize];
            stream.read_exact(&mut buf).map_err(|e| Error::TcpRead(e))?;
    
            match Syscall::decode(buf.as_ref()).map_err(|e| Error::Rpc(e))?.syscall {
                Some(SC::ReadKey(rk)) => {

                    let txn = DBENV.begin_ro_txn().unwrap();
                    let result = syscalls::ReadKeyResponse {
                        value: txn.get(self.db, &rk.key).ok().map(Vec::from),
                    }
                    .encode_to_vec();
                    let _ = txn.commit();
    
                    self.send_response(stream, result)?;
                },
                Some(SC::WriteKey(wk)) => {
                    let mut txn = DBENV.begin_rw_txn().unwrap();
                    let mut flags = WriteFlags::empty();
                    if let Some(f) = wk.flags {
                        flags = WriteFlags::from_bits(f).expect("bad flags");
                    }
                    
                    let result = syscalls::WriteKeyResponse {
                        success: txn
                            .put(self.db, &wk.key, &wk.value, flags)
                            .is_ok(),
                    }
                    .encode_to_vec();
                    let _ = txn.commit();
    
                    self.send_response(stream, result)?;
                },
                Some(SC::ReadDir(req)) => {
                    use lmdb::Cursor;
                    let mut keys: HashSet<Vec<u8>> = HashSet::new();
    
                    let txn = DBENV.begin_ro_txn().unwrap();
                    {
                        let mut dir = req.dir;
                        if !dir.ends_with(b"/") {
                            dir.push(b'/');
                        }
                        let mut cursor = txn.open_ro_cursor(self.db).or(Err(Error::RootfsNotExist))?.iter_from(&dir);
                        while let Some(Ok((key, _))) = cursor.next() {
                            if !key.starts_with(&dir) {
                                break
                            }
                            if let Some(entry) = key.split_at(dir.len()).1.split_inclusive(|c| *c == b'/').next() {
                                if !entry.is_empty() {
                                    keys.insert(entry.into());
                                }
                            }
                        }
                    }
                    let _ = txn.commit();
    
                    let result = syscalls::ReadDirResponse {
                        keys: keys.drain().collect(),
                    }.encode_to_vec();
    
                    self.send_response(stream, result)?;
                },
                Some(_) => {
                    // should never happen
                    error!("received unexpected syscall");
                },
                None => {
                    // Should never happen, so just ignore??
                    error!("received an unknown syscall");
                },
            }
        }
    }

    pub fn listen(&self) {
        let listener = TcpListener::bind(self.address.clone()).expect("listener failed to bind");
        debug!("DbServer started listening on: {:?}", self.address);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    debug!("New connection: {}", stream.peer_addr().unwrap());
                    if let Err(_e) = self.handle_request(stream){
                        return; // TODO: not ideal
                        // error!("handle request error: {:?}", e);
                    }
                }
                Err(e) => {
                    error!("stream error: {:?}", e);
                }
            }
        }
    }

    pub fn start_dbserver(db_server: DbServer) {
        std::thread::spawn(move || {
            db_server.listen();
        });
    }
}
