use std::net::{TcpListener, TcpStream};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use lmdb;
use lmdb::{Database, Transaction, WriteFlags};

use log::{error, debug};

use crate::syscalls;
use crate::labeled_fs::utils::get_new_dir_bytes;


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
    db: Arc<Mutex<Database>>,
    dbenv: Arc<Mutex<lmdb::Environment>>,
}

impl DbServer {

    pub fn new(address: String) -> Self {
        if !std::path::Path::new("storage").exists() {
            let _ = std::fs::create_dir("storage").unwrap();
        }
        
        let dbenv = lmdb::Environment::new()
            .set_map_size(100 * 1024 * 1024 * 1024)
            .open(std::path::Path::new("storage"))
            .unwrap();

        // Create the root directory object at key 0 if not already exists.
        // uses `NO_OVERWRITE` as the write flag to make sure that it
        // will be a noop if the root already exists at key 0. And we can safely ignore the
        // returned `Result` here which if an error is an KeyExist error.
        let default_db = dbenv.open_db(None).unwrap();
        let mut txn = dbenv.begin_rw_txn().unwrap();
        let root_uid: u64 = 0;
        let _ = txn.put(default_db, &root_uid.to_be_bytes(), &get_new_dir_bytes(), WriteFlags::NO_OVERWRITE);
        txn.commit().unwrap();

        DbServer { 
            address, 
            db: Arc::new(Mutex::new(default_db)), 
            dbenv: Arc::new(Mutex::new(dbenv)) 
        }
    }

    fn send_response(&self, mut stream: TcpStream, response: Vec<u8>) -> Result<(), Error> {
        use std::io::Write;

        stream.write_all(&(response.len() as u32).to_be_bytes()).map_err(|e| Error::TcpWrite(e))?;
        stream.write_all(response.as_ref()).map_err(|e| Error::TcpWrite(e))
    }

    fn handle_request(&self, stream: TcpStream) -> Result<(), Error> {
        use prost::Message;
        use std::io::Read;
        use syscalls::syscall::Syscall as SC;
        use syscalls::Syscall;

        debug!("handle request");

        loop {
            debug!("handle request in loop");
            let mut stream = stream.try_clone().unwrap();
            let mut lenbuf = [0;4];
            stream.read_exact(&mut lenbuf).map_err(|e| Error::TcpRead(e))?;
            let size = u32::from_be_bytes(lenbuf);
            let mut buf = vec![0u8; size as usize];
            stream.read_exact(&mut buf).map_err(|e| Error::TcpRead(e))?;
    
            match Syscall::decode(buf.as_ref()).map_err(|e| Error::Rpc(e))?.syscall {
                Some(SC::ReadKey(rk)) => {

                    let txn = self.dbenv.lock().unwrap().begin_ro_txn().unwrap();
                    let result = syscalls::ReadKeyResponse {
                        value: txn.get(*self.db.lock().unwrap(), &rk.key).ok().map(Vec::from),
                    }
                    .encode_to_vec();
                    let _ = txn.commit();
    
                    self.send_response(stream, result)?;
                },
                Some(SC::WriteKey(wk)) => {
                    let mut txn = self.dbenv.lock().unwrap().begin_rw_txn().unwrap();
                    let mut flags = WriteFlags::empty();
                    if let Some(f) = wk.flags {
                        flags = WriteFlags::from_bits(f).expect("bad flags");
                    }
                    
                    let result = syscalls::WriteKeyResponse {
                        success: txn
                            .put(*self.db.lock().unwrap(), &wk.key, &wk.value, flags)
                            .is_ok(),
                    }
                    .encode_to_vec();
                    let _ = txn.commit();
    
                    self.send_response(stream, result)?;
                },
                Some(SC::ReadDir(req)) => {
                    use lmdb::Cursor;
                    let mut keys: HashSet<Vec<u8>> = HashSet::new();
    
                    let txn = self.dbenv.lock().unwrap().begin_ro_txn().unwrap();
                    {
                        let mut dir = req.dir;
                        if !dir.ends_with(b"/") {
                            dir.push(b'/');
                        }
                        let mut cursor = txn.open_ro_cursor(*self.db.lock().unwrap()).or(Err(Error::RootfsNotExist))?.iter_from(&dir);
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
                Some(SC::CompareAndSwap(cas)) => {
                    let mut txn = self.dbenv.lock().unwrap().begin_rw_txn().unwrap();
                    let old = txn.get(*self.db.lock().unwrap(), &cas.key).ok().map(Into::into);
                    let res = if cas.expected == old {
                        let _ = txn.put(*self.db.lock().unwrap(), &cas.key, &cas.value, WriteFlags::empty());
                        Ok(())
                    } else {
                        Err(old)
                    };
                    txn.commit().unwrap();

                    let result = syscalls::WriteKeyResponse {
                        success: res.is_ok()
                    }.encode_to_vec();

                    self.send_response(stream, result)?;
                },
                Some(SC::Invoke(invoke)) => {
                    let result = syscalls::InvokeResponse {
                        success: invoke.function.eq("ping")
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
                    std::thread::spawn(move || {
                        debug!("New connection: {}", stream.peer_addr().unwrap());
                        match stream.read_timeout().unwrap() {
                            None => {
                                debug!("read timeout None");
                            },
                            Some(d) => {
                                debug!("read timeout {} ms", d.as_millis());
                            }
                        }
                        if let Err(_e) = self.handle_request(stream){
                            debug!("handle request returned error");
                            return; // TODO: not ideal
                            // error!("handle request error: {:?}", e);
                        }
                        debug!("done handling request");
                    });
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
