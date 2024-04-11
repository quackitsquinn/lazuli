# rsocks: A TCP Socket Library for rust aimed at game development

## Simple Usage
```rust
#[derive(Socketable)]
struct GameData {
    pub player_pos: (f32, f32),
    pub player_rot: f32,
    pub player_name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let mut server = Server::new_localhost(12345)?;
    let mut client = Client::new_localhost(12345)?;
    let mut data_stream: SocketStream<GameData> = server.create_stream();
    let mut data_stream2: SocketStream<GameData> = client.create_stream::<GameData>();
    let data = GameData {
        player_pos: (0.0, 0.0),
        player_rot: 0.0,
        player_name: "Player".to_string(),
    };
    data_stream.send(data.clone());
    let received_data = data_stream2.receive();
    assert_eq!(received_data, data);
}

```

## Data structure

```rust
#[repr(C)] // Prevent data being reordered and prevents weird trait stuff that would break the library
// zero, max, and half are just for checking if the data is actually a "packet"
// in this usage, a packet is a data structure that can be sent over the network
struct PacketHeader {
    zero: u8, // == 0u8
    packet_type: u16,
    max: u8, // == 255u8
    packet_length: u16,
    half: u8, // == 128u8
    checksum: u16, 
    
}
#[repr(C)] // Prevent data being reordered and prevents weird trait stuff that would break the library
struct RawPacket {
    packet_header: PacketHeader,
    data: Vec<u8>,
}
```

### Communication between Clients and Streams

- Each client has to be able to borrow the main client. 
  - This makes making multiple streams tricky because you have to borrow the client as mutable, which is not possible if you have multiple streams.
- I think a good solution would to have an Rc of the client, and then clone the Rc for each stream. 
  - This would allow for multiple streams to be created, but the client would still be mutable.


### Current planned support

- [x] Basic TCP communication
- [x] Communication without serialization
- [x] Cross platform support
- [ ] Pointer based data structures (vec, str, etc) (This is planned to be supported in the future)
  - This would be done by having custom serialization and deserialization functions for each data structure
- [ ] Cross language support (not planned)
