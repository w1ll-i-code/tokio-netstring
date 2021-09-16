use std::future::Future;
use std::marker::{PhantomPinned, Unpin};
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncWrite, ErrorKind, Result};

pub(crate) fn write_netstring<'a, A>(writer: &'a mut A, buf: &'a [u8]) -> WriteMessage<'a, A>
where
    A: AsyncWrite + Unpin + ?Sized,
{
    const NETSTRING_MAX_OVERHEAD: usize = 22; // usize::MAX.to_string().len() + b':'.len() + b','.len()

    let mut buffer = Vec::with_capacity(buf.len() + NETSTRING_MAX_OVERHEAD);
    buffer.extend_from_slice(buf.len().to_string().as_bytes());
    buffer.extend_from_slice(&[b':']);
    buffer.extend_from_slice(buf);
    buffer.extend_from_slice(&[b',']);

    WriteMessage {
        writer,
        buf: buffer,
        prog: 0,
        _pin: Default::default(),
    }
}

pin_project! {
    /// Creates a future which will read exactly enough bytes to fill `buf`,
    /// returning an error if EOF is hit sooner.
    ///
    /// On success the number of bytes is returned
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct WriteMessage<'a, A: ?Sized> {
        writer: &'a mut A,
        buf: Vec<u8>,
        prog: usize,
        // Make this future `!Unpin` for compatibility with async trait methods.
        #[pin]
        _pin: PhantomPinned,
    }
}

impl<A> Future for WriteMessage<'_, A>
where
    A: AsyncWrite + Unpin + ?Sized,
{
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let me = self.project();

        loop {
            let n = ready_and_ok!(Pin::new(&mut *me.writer).poll_write(cx, &me.buf[*me.prog..]));
            *me.prog += n;

            if *me.prog == me.buf.len() {
                return Poll::Ready(Ok(*me.prog));
            }

            if n == 0 {
                return Poll::Ready(Err(ErrorKind::WriteZero.into()));
            }
        }
    }
}
