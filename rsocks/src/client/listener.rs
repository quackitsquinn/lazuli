use std::{
    io::{self, stdout, Write},
    net::TcpStream,
    sync::{atomic::AtomicBool, Arc},
};

use log::error;

use crate::{ArcMutex, IOResult};

use super::{input, StreamCollection};

pub struct SocketListener {
    socket: ArcMutex<TcpStream>,
    streams: ArcMutex<StreamCollection>,
    thread: Option<std::thread::JoinHandle<IOResult<()>>>,
    should_close: Arc<AtomicBool>,
    error: Option<io::Error>,
}

impl SocketListener {
    pub fn new(socket: ArcMutex<TcpStream>, streams: ArcMutex<StreamCollection>) -> Self {
        Self {
            socket,
            streams,
            thread: None,
            should_close: Arc::new(AtomicBool::new(false)),
            error: None,
        }
    }
    pub fn run(&mut self) -> IOResult<()> {
        let run = self.should_close.clone();
        let socket = self.socket.clone();
        // Set the socket to non-blocking mode. This is EXTREMELY important for the listener to work.
        // If it is blocking, the thread will never exit, and the program will hang.
        socket.lock().unwrap().set_nonblocking(true)?;
        let streams = self.streams.clone();
        let thread = std::thread::spawn(move || Self::run_thread(run, socket, streams));
        self.thread = Some(thread);
        Ok(())
    }
    fn run_thread(
        should_close: Arc<AtomicBool>,
        socket: ArcMutex<TcpStream>,
        streams: ArcMutex<StreamCollection>,
    ) -> IOResult<()> {
        while !should_close.load(std::sync::atomic::Ordering::Acquire) {
            match Self::thread_inner(should_close.clone(), socket.clone(), streams.clone()) {
                Ok(_) => {}
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        error!("Error in listener thread: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn thread_inner(
        should_close: Arc<AtomicBool>,
        socket: ArcMutex<TcpStream>,
        streams: ArcMutex<StreamCollection>,
    ) -> IOResult<()> {
        let mut stream = socket.lock().unwrap();
        let header = input::input_header(&mut *stream)?;
        let mut would_block = true;
        while would_block {
            match input::input_data(&mut *stream, &header) {
                Err(e) => {
                    // if the thread is closing, return.
                    if should_close.load(std::sync::atomic::Ordering::Acquire) {
                        return Ok(());
                    }
                    if e.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    return Err(e);
                }
                Ok(data) => {
                    input::verify_checksum(&header, data.as_slice())?;
                    let mut streams = streams.lock().unwrap();
                    if let Some(info) = streams.get_mut(&header.id()) {
                        info.push(data, header)?;
                    } else {
                        error!("Stream not found: {}", header.id());
                    }
                    would_block = false;
                }
            }
        }
        Ok(())
    }
    pub fn error(&self) -> Option<io::Error> {
        if let Some(err) = &self.error {
            // Make a clone of the error. (I don't know why io::Error doesn't implement Clone, but it's probably for a good reason.)
            Some(io::Error::new(err.kind(), err.to_string()))
        } else {
            None
        }
    }

    pub fn stop(&mut self) -> IOResult<()> {
        self.should_close
            .store(true, std::sync::atomic::Ordering::Release);
        self.thread.take().unwrap().join().unwrap()
    }
}

impl Drop for SocketListener {
    fn drop(&mut self) {
        if self.thread.is_some() {
            let _ = self.stop();
        }
    }
}

#[cfg(test)]
mod tests {

    // TODO: add tests or a macro to generate tests.
}
