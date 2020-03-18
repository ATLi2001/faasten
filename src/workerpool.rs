//! A fixed size pool (maybe slightly below max, max being total memory/120MB)
//! Acquire a free worker from a pool. This should always succeed because we
//! should not run out of worker threads.
//! A worker takes a reqeust and finds a VM to execute it. 

use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, Receiver, SendError};

use log::{error, warn, info};

use crate::worker::Worker;
use crate::request::Request;
use crate::message::Message;
use crate::controller::Controller;

pub struct WorkerPool {
    pool: Vec<Worker>,
    req_sender: Sender<Message>,
    controller: Arc<Controller>,
}

impl WorkerPool {
    pub fn new(controller: Arc<Controller>) -> WorkerPool {
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));


        let pool_size = controller.total_mem/128;
        let mut pool = Vec::with_capacity(pool_size);

        for _ in 0..pool_size {
            pool.push(Worker::new(rx.clone(), controller.clone()));
        }

        WorkerPool {
            pool: pool,
            req_sender: tx,
            controller: controller,
        }
    }

    pub fn send_req(&self, req: Request, rsp_sender: Sender<Message>) {
        self.req_sender.send(Message::Request(req, rsp_sender));
    }

    pub fn shutdown(self) {

        for _ in &self.pool {
            self.req_sender.send(Message::Shutdown);
        }

        for w in self.pool {
            let id = w.thread.thread().id();
            if let Err(e) = w.thread.join() {
                error!("worker thread {:?} panicked {:?}", id, e);
            }
        }

        self.controller.shutdown();
    }
}
