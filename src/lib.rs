//! # rsocks: A TCP socket library oriented for game design
//!

use std::sync::Arc;

pub use rsocks;
pub use rsocks_derive;

//#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use rsocks::Sendable;

    #[derive(rsocks_derive::Sendable)]
    struct TestSendable {
        a: u32,
        b: u32,
    }

    #[test]
    fn test_sendable() {
        let test = TestSendable { a: 1, b: 2 };
        let data = test.send();
        let mut p = Cursor::new(data);
        let test2 = TestSendable::recv(&mut p).unwrap();
        assert_eq!(test.a, test2.a);
        assert_eq!(test.b, test2.b);
    }
}
