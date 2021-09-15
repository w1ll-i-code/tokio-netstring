#[macro_export]
macro_rules! ready {
    ($e:expr) => {
        match $e {
            Poll::Ready(t) => t,
            Poll::Pending => return Poll::Pending,
        }
    };
}

#[macro_export]
macro_rules! ready_and_ok {
    ($e:expr) => {
        match ready!($e) {
            Ok(val) => val,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

#[macro_export]
macro_rules! bytes_read {
    ($e:expr) => {
        match $e.filled().len() {
            0 => return Poll::Ready(Err(eof())),
            len => len,
        }
    };
}

// #[macro_export]
// macro_rules! read_byte {
//     ($e:expr) => {
//         let mut buf = [0;1];
//         let buf = BufReader::new(&mut buf);
//         let _ = bytes_read!(read_and_ok!(Pin::new().poll_read(&mut buf)));
//         buf[0]
//     };
// }