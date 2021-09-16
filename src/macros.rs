#![macro_use]

macro_rules! ready {
    ($e:expr) => {
        match $e {
            Poll::Ready(t) => t,
            Poll::Pending => return Poll::Pending,
        }
    };
}

macro_rules! ready_and_ok {
    ($e:expr) => {
        match ready!($e) {
            Ok(val) => val,
            Err(err) => return Poll::Ready(Err(err)),
        }
    };
}

macro_rules! bytes_read {
    ($e:expr) => {
        match $e.filled().len() {
            0 => return Poll::Ready(Err(eof())),
            len => len,
        }
    };
}

macro_rules! read_byte {
    ($reader:expr, $cx:expr) => {{
        let mut byte_buf = [0; 1];
        let mut read_buf = ReadBuf::new(&mut byte_buf);
        ready_and_ok!(Pin::new(&mut *$reader).poll_read($cx, &mut read_buf));
        bytes_read!(read_buf);
        byte_buf[0]
    }};
}
