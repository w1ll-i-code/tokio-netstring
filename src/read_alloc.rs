use std::future::Future;
use std::marker::{PhantomPinned, Unpin};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, Error, ErrorKind, ReadBuf, Result};

// usize::MAX.to_string().len() + one byte separator
const MAX_LENGTH: usize = 21;

#[derive(Debug)]
enum State {
    Ready,
    ReadLength([u8; MAX_LENGTH], usize),
    ParseLength([u8; MAX_LENGTH], usize),
    ParseSeparator(usize, u8),
    AllocateMemory(usize),
    ReadMessage(Vec<u8>, usize),
    ParseTerminator(Vec<u8>),
}

pub(crate) fn read_netstring_alloc<A>(reader: &mut A) -> ReadMessageAlloc<'_, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    ReadMessageAlloc {
        reader,
        state: State::Ready,
        _pin: PhantomPinned,
    }
}

pin_project! {
    /// Creates a future which will read exactly one message in the netstring format
    /// returning an error if EOF is hit sooner.
    ///
    /// On success the number of bytes is returned
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ReadMessageAlloc<'a, A: ?Sized> {
        reader: &'a mut A,
        state: State,
        // Make this future `!Unpin` for compatibility with async trait methods.
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<A> Future for ReadMessageAlloc<'_, A>
where
    A: AsyncRead + Unpin + ?Sized,
{
    type Output = Result<Vec<u8>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Vec<u8>>> {
        let me = self.project();

        loop {
            match me.state {
                //initialize the state machine
                State::Ready => {
                    *me.state = State::ReadLength([0; MAX_LENGTH], 0);
                }

                //read the length of the netstring, one byte at a time
                State::ReadLength(buf, prog) => {
                    buf[*prog] = read_byte!(me.reader, cx);
                    match *prog == MAX_LENGTH - 1 || !buf[*prog].is_ascii_digit() {
                        true => *me.state = State::ParseLength(*buf, *prog),
                        false => *prog += 1,
                    }
                }

                //parse the length, the last byte in the buffer is the first non-ascii digit.
                State::ParseLength(buf, len) => {
                    match String::from_utf8_lossy(&buf[..*len]).parse() {
                        Ok(msg_len) => *me.state = State::ParseSeparator(msg_len, buf[*len]),
                        Err(_) => return integer_overflow(),
                    }
                }

                //verify that length and message are separated by a ':'
                State::ParseSeparator(len, separator) => match *separator {
                    b':' => *me.state = State::AllocateMemory(*len),
                    sep => return wrong_separator(sep),
                },

                State::AllocateMemory(size) => *me.state = State::ReadMessage(vec![0; *size], 0),

                //read the message from the stream
                State::ReadMessage(buf, prog) => {
                    if *prog == (buf.capacity()) {
                        *me.state = State::ParseTerminator(std::mem::take(buf));
                    } else {
                        let read = {
                            let mut reader = ReadBuf::new(&mut buf[*prog..]);
                            ready_and_ok!(Pin::new(&mut *me.reader).poll_read(cx, &mut reader));
                            bytes_read!(reader)
                        };
                        *prog += read;
                    }
                }

                //verify that the message is terminated with a ','
                State::ParseTerminator(buf) => {
                    return match read_byte!(me.reader, cx) {
                        b',' => return Poll::Ready(Ok(std::mem::take(buf))),
                        term => wrong_terminator(term),
                    }
                }
            }
        }
    }
}

fn eof() -> Error {
    Error::new(ErrorKind::UnexpectedEof, "early eof")
}

fn integer_overflow() -> Poll<Result<Vec<u8>>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        "ERROR: Integer overflow while parsing message length.".to_string(),
    )))
}

fn wrong_separator(separator: u8) -> Poll<Result<Vec<u8>>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "ERROR: Expected separator ':' but found {} instead",
            separator as char
        ),
    )))
}

fn wrong_terminator(terminator: u8) -> Poll<Result<Vec<u8>>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "ERROR: Expected terminator ',' but found {} instead",
            terminator as char
        ),
    )))
}
