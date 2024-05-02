use std::{
    collections::HashMap,
    io::{self, stdout, Write},
    net::TcpStream,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use crate::{ArcMutex, IOResult, TcpClient};

use super::StreamCollection;

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
        let thread = std::thread::spawn(move || Self::run_inner(run, socket, streams));
        self.thread = Some(thread);
        Ok(())
    }
    fn run_inner(
        should_close: Arc<AtomicBool>,
        socket: ArcMutex<TcpStream>,
        streams: ArcMutex<StreamCollection>,
    ) -> IOResult<()> {
        let mut client =
            TcpClient::from_arcmutex_socket(socket.clone()).with_streams(streams.clone());
        while !should_close.load(std::sync::atomic::Ordering::Acquire) {
            match client.recv() {
                Ok(_) => {}
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    } else {
                        return Err(err);
                    }
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
    use std::{thread, time::Duration};

    use crate::{client::test_utils::make_client_server_pair, stream::Stream};

    #[test]
    fn test_listener() {
        let (mut client, mut server) = make_client_server_pair();
        let mut stream: Stream<u32> = client.stream();
        client.listen();
        println!("Sending data...");
        server.send(&32u32).unwrap();

        let mut i = 0;
        let mut res = None;
        while res.is_none() {
            println!("Waiting for data...");
            thread::sleep(Duration::from_millis(100));
            if i > 100 {
                panic!("Stream did not receive data in time.");
            }
            i += 1;
            res = stream.get();
        }
        assert_eq!(res.unwrap(), 32);
    }
}