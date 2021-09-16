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
    DropMessage(usize),
    ParseTerminator,
}

pub(crate) fn drop_netstring<A>(reader: &mut A) -> DropMessage<A>
    where A: AsyncRead + Unpin + ?Sized,
    {
        DropMessage {
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
    pub struct DropMessage<'a, A: ?Sized> {
        reader: &'a mut A,
        state: State,
        // Make this future `!Unpin` for compatibility with async trait methods.
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<A> Future for DropMessage<'_, A>
    where
        A: AsyncRead + Unpin + ?Sized,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
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
                    b':' => *me.state = State::DropMessage(*len),
                    sep => return wrong_separator(sep),
                },

                //read the message from the stream
                State::DropMessage(remaining) => match *remaining {
                    0 => *me.state = State::ParseTerminator,
                    _ => {
                        let read = {
                            let mut buf = [0;1024];
                            let mut buf = ReadBuf::new(&mut buf);
                            let mut reader = buf.take(*remaining);
                            ready_and_ok!(Pin::new(&mut *me.reader).poll_read(cx, &mut reader));
                            bytes_read!(reader)
                        };
                        *remaining -= read;
                    }
                },

                //verify that the message is terminated with a ','
                State::ParseTerminator => {
                    return match read_byte!(me.reader, cx) {
                        b',' => Poll::Ready(Ok(())),
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

fn integer_overflow() -> Poll<Result<()>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        "ERROR: Integer overflow while parsing message length.".to_string(),
    )))
}

fn wrong_separator(separator: u8) -> Poll<Result<()>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "ERROR: Expected separator ':' but found {} instead",
            separator as char
        ),
    )))
}

fn wrong_terminator(terminator: u8) -> Poll<Result<()>> {
    Poll::Ready(Err(Error::new(
        ErrorKind::InvalidData,
        format!(
            "ERROR: Expected terminator ',' but found {} instead",
            terminator as char
        ),
    )))
}
