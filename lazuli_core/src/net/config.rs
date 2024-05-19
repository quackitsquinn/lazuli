/// Config flags for the underlying socket of a client.
pub struct SocketConfig {
    /// Whether the socket should be blocking.
    pub blocking: Option<bool>,
    /// The read timeout for the socket.
    pub read_timeout: Option<std::time::Duration>,
    /// The write timeout for the socket.
    pub write_timeout: Option<std::time::Duration>,
    /// The time-to-live for the socket.
    pub ttl: Option<u32>,
    /// Whether the socket should have the Nagle algorithm disabled
    pub nodelay: Option<bool>,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            blocking: None,
            read_timeout: None,
            write_timeout: None,
            ttl: None,
            nodelay: None,
        }
    }
}

impl SocketConfig {
    /// Creates a new `SocketConfig` with all fields set to `None`.
    /// This is equivalent to `SocketConfig::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies the configuration to the given socket. Any fields that are `None` are ignored.
    pub fn apply_stream(&self, socket: &std::net::TcpStream) -> std::io::Result<()> {
        if let Some(blocking) = self.blocking {
            socket.set_nonblocking(!blocking)?;
        }
        if let Some(read_timeout) = self.read_timeout {
            socket.set_read_timeout(Some(read_timeout))?;
        }
        if let Some(write_timeout) = self.write_timeout {
            socket.set_write_timeout(Some(write_timeout))?;
        }
        if let Some(ttl) = self.ttl {
            socket.set_ttl(ttl)?;
        }
        if let Some(nodelay) = self.nodelay {
            socket.set_nodelay(nodelay)?;
        }
        Ok(())
    }

    /// Applies the configuration to the given listener. Any fields that are `None` are ignored.
    pub fn apply_listener(&self, listener: &std::net::TcpListener) -> std::io::Result<()> {
        if let Some(blocking) = self.blocking {
            listener.set_nonblocking(!blocking)?;
        }
        if let Some(ttl) = self.ttl {
            listener.set_ttl(ttl)?;
        }
        Ok(())
    }

    /// Sets the blocking flag for the socket.
    pub fn blocking(mut self, blocking: bool) -> Self {
        self.blocking = Some(blocking);
        self
    }

    /// Sets the read timeout for the socket.
    pub fn read_timeout(mut self, read_timeout: std::time::Duration) -> Self {
        self.read_timeout = Some(read_timeout);
        self
    }

    /// Sets the write timeout for the socket.
    pub fn write_timeout(mut self, write_timeout: std::time::Duration) -> Self {
        self.write_timeout = Some(write_timeout);
        self
    }

    /// Sets the time-to-live for the socket.
    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Sets the nodelay flag for the socket.
    pub fn nodelay(mut self, nodelay: bool) -> Self {
        self.nodelay = Some(nodelay);
        self
    }
}
