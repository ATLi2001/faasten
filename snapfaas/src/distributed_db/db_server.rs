use std::net::{TcpListener, TcpStream};

use lmdb::Database;

use log::{error, debug};

use crate::syscalls;
use crate::labeled_fs::DBENV;


#[derive(Debug)]
pub enum Error {
    ProcessSpawn(std::io::Error),
    Rpc(prost::DecodeError),
    VsockListen(std::io::Error),
    VsockWrite(std::io::Error),
    VsockRead(std::io::Error),
    HttpReq(reqwest::Error),
    AuthTokenInvalid,
    AuthTokenNotExist,
    KernelNotExist,
    RootfsNotExist,
    AppfsNotExist,
    LoadDirNotExist,
    DB(lmdb::Error),
    BlobError(std::io::Error),
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
        DbServer { address: address, db: default_db }
    }

    fn send_response(&self, mut stream: TcpStream, response: Vec<u8>) -> Result<(), Error> {
        use std::io::Write;

        stream.write_all(&(response.len() as u32).to_be_bytes()).map_err(|e| Error::VsockWrite(e))?;
        stream.write_all(response.as_ref()).map_err(|e| Error::VsockWrite(e))
    }

    fn handle_request(&self, mut stream: TcpStream) -> Result<(), Error> {
        use lmdb::{Transaction, WriteFlags};
        use prost::Message;
        use std::io::Read;
        use syscalls::syscall::Syscall as SC;
        use syscalls::Syscall;

        let mut lenbuf = [0;4];
        stream.read_exact(&mut lenbuf).map_err(|e| Error::VsockRead(e))?;
        let size = u32::from_be_bytes(lenbuf);
        let mut buf = vec![0u8; size as usize];
        stream.read_exact(&mut buf).map_err(|e| Error::VsockRead(e))?;

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
                let result = syscalls::WriteKeyResponse {
                    success: txn
                        .put(self.db, &wk.key, &wk.value, WriteFlags::empty())
                        .is_ok(),
                }
                .encode_to_vec();
                let _ = txn.commit();

                self.send_response(stream, result)?;
            },
            // Some(SC::ReadDir(req)) => {
            //     use lmdb::Cursor;
            //     let mut keys: HashSet<Vec<u8>> = HashSet::new();

            //     let txn = DBENV.begin_ro_txn().unwrap();
            //     {
            //         let mut dir = req.dir;
            //         if !dir.ends_with(b"/") {
            //             dir.push(b'/');
            //         }
            //         let mut cursor = txn.open_ro_cursor(default_db).or(Err(Error::RootfsNotExist))?.iter_from(&dir);
            //         while let Some(Ok((key, _))) = cursor.next() {
            //             if !key.starts_with(&dir) {
            //                 break
            //             }
            //             if let Some(entry) = key.split_at(dir.len()).1.split_inclusive(|c| *c == b'/').next() {
            //                 if !entry.is_empty() {
            //                     keys.insert(entry.into());
            //                 }
            //             }
            //         }
            //     }
            //     let _ = txn.commit();

            //     let result = syscalls::ReadDirResponse {
            //         keys: keys.drain().collect(),
            //     }.encode_to_vec();
            //     send_response(result)?;
            // },
            // Some(SC::FsRead(req)) => {
            //     let result = syscalls::ReadKeyResponse {
            //         value: labeled_fs::read(req.path.as_str(), &mut self.current_label).ok(),
            //     }
            //     .encode_to_vec();

            //     send_response(result)?;
            // },
            // Some(SC::FsWrite(req)) => {
            //     let result = syscalls::WriteKeyResponse {
            //         success: labeled_fs::write(req.path.as_str(), req.data, &mut self.current_label).is_ok(),
            //     }
            //     .encode_to_vec();

            //     send_response(result)?;
            // },
            // Some(SC::FsCreateDir(req)) => {
            //     let label = proto_label_to_dc_label(req.label.expect("label"));
            //     let result = syscalls::WriteKeyResponse {
            //         success: labeled_fs::create_dir(
            //             req.base_dir.as_str(), req.name.as_str(), label, &mut self.current_label
            //         ).is_ok(),
            //     }
            //     .encode_to_vec();

            //     send_response(result)?;
            // },
            // Some(SC::FsCreateFile(req)) => {
            //     let label = proto_label_to_dc_label(req.label.expect("label"));
            //     let result = syscalls::WriteKeyResponse {
            //         success: labeled_fs::create_file(
            //             req.base_dir.as_str(), req.name.as_str(), label, &mut self.current_label
            //         ).is_ok(),
            //     }
            //     .encode_to_vec();

            //     send_response(result)?;
            // },
            // Some(SC::CreateBlob(_cb)) => {
            //     if let Ok(newblob) = self.blobstore.create().map_err(|_e| Error::AppfsNotExist) {
            //         self.max_blob_id += 1;
            //         self.create_blobs.insert(self.max_blob_id, newblob);

            //         let result = syscalls::BlobResponse {
            //             success: true,
            //             fd: self.max_blob_id,
            //             data: Vec::new(),
            //         };
            //         send_response(result.encode_to_vec())?;
            //     } else {
            //         let result = syscalls::BlobResponse {
            //             success: false,
            //             fd: 0,
            //             data: Vec::new(),
            //         };
            //         send_response(result.encode_to_vec())?;
            //     }
            // },
            // Some(SC::WriteBlob(wb)) => {
            //     let result = if let Some(newblob) = self.create_blobs.get_mut(&wb.fd) {
            //         let data = wb.data.as_ref();
            //         if newblob.write_all(data).is_ok() {
            //             syscalls::BlobResponse {
            //                 success: true,
            //                 fd: wb.fd,
            //                 data: Vec::new(),
            //             }
            //         } else {
            //             syscalls::BlobResponse {
            //                 success: false,
            //                 fd: wb.fd,
            //                 data: Vec::from("Failed to write"),
            //             }
            //         }
            //     } else {
            //         syscalls::BlobResponse {
            //             success: false,
            //             fd: wb.fd,
            //             data: Vec::from("Blob doesn't exist"),
            //         }
            //     };
            //     send_response(result.encode_to_vec())?;
            // },
            // Some(SC::FinalizeBlob(fb)) => {
            //     let result = if let Some(mut newblob) = self.create_blobs.remove(&fb.fd) {
            //         let blob = newblob.write_all(&fb.data).and_then(|_| self.blobstore.save(newblob))?;
            //         syscalls::BlobResponse {
            //             success: true,
            //             fd: fb.fd,
            //             data: Vec::from(blob.name),
            //         }
            //     } else {
            //         syscalls::BlobResponse {
            //             success: false,
            //             fd: fb.fd,
            //             data: Vec::from("Blob doesn't exist"),
            //         }
            //     };
            //     send_response(result.encode_to_vec())?;
            // },
            // Some(SC::OpenBlob(ob)) => {
            //     let result = if let Ok(file) = self.blobstore.open(ob.name) {
            //         self.max_blob_id += 1;
            //         self.blobs.insert(self.max_blob_id, file);
            //         syscalls::BlobResponse {
            //             success: true,
            //             fd: self.max_blob_id,
            //             data: Vec::new(),
            //         }
            //     } else {
            //         syscalls::BlobResponse {
            //             success: false,
            //             fd: 0,
            //             data: Vec::new(),
            //         }
            //     };
            //     send_response(result.encode_to_vec())?;
            // },
            // Some(SC::ReadBlob(rb)) => {
            //     let result = if let Some(file) = self.blobs.get_mut(&rb.fd) {
            //         let mut buf = Vec::from([0; 4096]);
            //         let limit = std::cmp::min(rb.length.unwrap_or(4096), 4096) as usize;
            //         if let Some(offset) = rb.offset {
            //             file.seek(std::io::SeekFrom::Start(offset))?;
            //         }
            //         if let Ok(len) = file.read(&mut buf[0..limit]) {
            //             buf.truncate(len);
            //             syscalls::BlobResponse {
            //                 success: true,
            //                 fd: rb.fd,
            //                 data: buf,
            //             }
            //         } else {
            //             syscalls::BlobResponse {
            //                 success: false,
            //                 fd: rb.fd,
            //                 data: Vec::new(),
            //             }
            //         }
            //     } else {
            //             syscalls::BlobResponse {
            //                 success: false,
            //                 fd: rb.fd,
            //                 data: Vec::new(),
            //             }
            //     };
            //     send_response(result.encode_to_vec())?;
            // },
            // Some(SC::CloseBlob(cb)) => {
            //     let result = if self.blobs.remove(&cb.fd).is_some() {
            //         syscalls::BlobResponse {
            //             success: true,
            //             fd: cb.fd,
            //             data: Vec::new(),
            //         }
            //     } else {
            //         syscalls::BlobResponse {
            //             success: false,
            //             fd: cb.fd,
            //             data: Vec::new(),
            //         }
            //     };
            //     send_response(result.encode_to_vec())?;
            // },
            Some(_) => {
                // should never happen
                error!("received unexpected syscall");
            },
            None => {
                // Should never happen, so just ignore??
                error!("received an unknown syscall");
            },
        }
        Ok(())
    }

    pub fn listen(&self) {
        let listener = TcpListener::bind(self.address.clone()).expect("listener failed to bind");
        debug!("DbServer started listening on: {:?}", self.address);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    debug!("New connection: {}", stream.peer_addr().unwrap());
                    if let Err(e) = self.handle_request(stream){
                        error!("handle request error: {:?}", e);
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
