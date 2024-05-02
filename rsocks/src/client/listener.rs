use std::{
    collections::HashMap,
    f32::consts::E,
    io::{self, stdout, Write},
    net::TcpStream,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use crate::{ArcMutex, IOResult, TcpClient};

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
                        let mut stdout = stdout();
                        writeln!(stdout, "Error in listener thread: {}", e).unwrap();
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
        let mut header = input::input_header(&mut *stream)?;
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
                        let mut stdout = stdout();
                        writeln!(stdout, "Stream not found: {}", header.id()).unwrap();
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
    use std::{thread, time::Duration};

    use crate::{client::test_utils::make_client_server_pair, stream::Stream};

    #[test]
    fn test_listener() {
        let (mut client, mut server) = make_client_server_pair();
        let mut stream: Stream<u32> = client.stream();
        let mut string_stream: Stream<String> = client.stream();
        client.listen();
        println!("Sending data...");
        server.send(&32u32).unwrap();
        server.send(&"Hello, world!".to_string()).unwrap();

        let mut i = 0;
        let mut u32_result = None;
        let mut string_result = None;
        while i < 10 {
            if let Some(data) = stream.get() {
                u32_result = Some(data);
            }
            if let Some(data) = string_stream.get() {
                string_result = Some(data);
            }
            if u32_result.is_some() && string_result.is_some() {
                break;
            } else {
                println!("listener err_state: {:?}", client.error());
            }
            thread::sleep(Duration::from_millis(10));
            i += 1;
        }
        assert_eq!(u32_result.unwrap(), 32);
        assert_eq!(string_result.unwrap(), "Hello, world!");
    }
}
