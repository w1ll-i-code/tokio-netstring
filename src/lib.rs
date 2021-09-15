use tokio::io::{AsyncRead, AsyncWrite};

mod macros;
mod read;
mod read_alloc;
mod write;
mod drop;

pub trait NetstringReader: AsyncRead {
    fn read_netstring<'a>(&'a mut self, buf: &'a mut [u8]) -> read::ReadMessage<'a, Self>
    where
        Self: Unpin,
    {
        read::read_netstring(self, buf)
    }

    fn read_netstring_alloc(&mut self) -> read_alloc::ReadMessageAlloc<Self>
    where
        Self: Unpin,
    {
        read_alloc::read_netstring_alloc(self)
    }

    fn drop_netstring(&mut self) -> drop::DropMessage<Self>
    where
        Self: Unpin,
    {
        drop::drop_netstring(self)
    }
}

pub trait NetstringWriter: AsyncWrite {
    fn write_netstring<'a>(&'a mut self, buf: &'a [u8]) -> write::WriteMessage<'a, Self>
    where
        Self: Unpin,
    {
        write::write_netstring(self, buf)
    }
}

impl<R: AsyncRead + ?Sized> NetstringReader for R {}

impl<W: AsyncWrite + ?Sized> NetstringWriter for W {}
