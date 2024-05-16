pub use lazuli_core::*;
pub use lazuli_derive::Sendable;

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use lazuli_core::Sendable;

    #[derive(lazuli_derive::Sendable, PartialEq, Debug)]
    struct TestSendable {
        uint32_1: u32,
        uint32_2: u32,
    }

    impl TestSendable {
        fn new(uint32_1: u32, uint32_2: u32) -> Self {
            Self { uint32_1, uint32_2 }
        }
    }

    #[test]
    fn test_sendable() {
        let test = TestSendable::new(4, 32);
        let data = test.send();
        let mut p = Cursor::new(data);
        let test2 = TestSendable::recv(&mut p).unwrap();
        assert_eq!(test, test2);
    }

    #[derive(lazuli_derive::Sendable, PartialEq, Debug)]
    struct TestSendable2 {
        sendable: TestSendable,
        vec_u32: Vec<u32>,
        int32: i32,
    }

    impl TestSendable2 {
        // base is used to generate unique values for each field
        fn new(base: u8) -> Self {
            let sendable = TestSendable::new(base as u32, (base * 2) as u32);
            let vec_u32 = {
                let mut vec = Vec::new();
                for i in 0..10 {
                    vec.push(i + base as u32);
                }
                vec
            };
            let int32 = (base as i32) << 4;
            Self {
                sendable,
                vec_u32,
                int32,
            }
        }
    }

    #[test]
    fn test_sendable2() {
        let test = TestSendable2 {
            sendable: TestSendable {
                uint32_1: 1,
                uint32_2: 2,
            },
            vec_u32: vec![1, 2, 3],
            int32: 4,
        };
        let data = test.send();
        let mut p = Cursor::new(data);
        let received = TestSendable2::recv(&mut p).unwrap();
        assert!(p.position() == p.get_ref().len() as u64);
        assert_eq!(test, received);
    }

    #[derive(lazuli_derive::Sendable, Debug, PartialEq)]
    struct TestSendable3 {
        a: TestSendable2,
        b: Vec<u32>,
        c: i32,
        d: Vec<TestSendable>,
    }

    #[test]
    fn test_sendable3() {
        let constructed = TestSendable3 {
            a: TestSendable2::new(12),
            b: vec![1, 2, 3],
            c: 4,
            d: vec![TestSendable::new(1, 2), TestSendable::new(3, 4)],
        };
        let data = constructed.send();
        let mut p = Cursor::new(data);
        let received = TestSendable3::recv(&mut p).unwrap();
        assert!(p.position() == p.get_ref().len() as u64);
        assert_eq!(constructed, received);
    }

    #[derive(lazuli_derive::Sendable, Debug, PartialEq)]
    struct TupleTest(u32, u32);

    #[test]
    fn test_tuple() {
        let test = TupleTest(1, 2);
        let data = test.send();
        let mut p = Cursor::new(data);
        let test2 = TupleTest::recv(&mut p).unwrap();
        assert_eq!(test, test2);
    }
}
