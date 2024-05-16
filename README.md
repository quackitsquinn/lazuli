# lazuli: A rust socket library for consistent, quick, and easy data transfer

lazuli is a socket library that provides a simple interface for sending and receiving data over a network. It is designed to be easy to use, fast, and reliable. lazuli is built on top of the standard Rust `std::net` library, and provides a more user-friendly API for working with sockets.

## Features

- Simple API for sending and receiving data
- Support for TCP sockets
- Non-blocking I/O
- Cross-platform support
- Standard error types for easy error handling


## Usage

Lazuli uses a simple API that consists of a `Client`, `Server`, and `Stream<T>` struct. The `Client` struct is used to connect to a server and send data, the `Server` struct is used to listen for incoming connections, and the `Stream<T>` struct is used to receive data.

Here is an example of how to use lazuli to send and receive data:

```rust

let client = Client::new(("127.0.0.1", 8080))

let stream = client.stream::<String>();

client.send("Hello, world!".to_string());

let data = stream.recv().unwrap();

println!("Received data: {}", data);

```

## Contributing

Contributions are welcome! If you would like to contribute to lazuli, please open an issue or submit a pull request. If you are submitting a pull request, please make sure to run a `cargo fmt` before submitting.

## License

lazuli is licensed under the GNU GPL v3.0. See the [LICENSE](LICENSE) file for more information.
