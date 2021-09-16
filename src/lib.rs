
mod macros;
mod read;
mod read_alloc;
mod write;
mod drop;

/// # NOTICE
/// This is the very first release and my first project in rust. While I tried to test everything, I
/// can't guarantee it will always work. Feedback is appreciated.
mod tokio_netstring {
    use tokio::io::{AsyncRead, AsyncWrite};
    use crate::*;

    /// The `NetstringReader` trait allows you to read one netstring at a time from any stream that has
    /// `AsyncRead` implemented. No implementation is thread-safe and multiple simultaneous reads
    /// can corrupt the message irreparably.
    pub trait NetstringReader: AsyncRead {
        /// This method allows to read one netstring into the buffer given. It is advisable to use
        /// this Trait on a [tokio::io::BufReader] to avoid repeated system calls during parsing.
        ///
        /// # Usage
        /// ```no_exec
        /// use tokio_netstring::NetstringReader;
        ///
        /// let buf = [0; 1024];
        /// let len: usize = stream.read_netstring(&mut buf).await.unwrap();
        /// let buf: &[u8] = &buf[..len];
        /// ```
        ///
        /// # Errors
        /// This method returns a `tokio::io::Result` which is a re-export from `std::io::Result`.
        ///
        /// ## ErrorKind::UnexpectedEof
        /// This error kind is returned, if the stream got closed, before a Netstring could be fully read.
        ///
        /// ## ErrorKind::BrokenPipe
        /// This error type indicates that the buffer provided is to small for the netstring to fit in.
        /// In the current implementation this error is irrecoverable as it has corrupted the stream.
        /// Future implementations may allow to recover from this.
        ///
        /// ## ErrorKind::InvalidData
        /// This error can be returned on three occasions:
        ///
        /// 1. The size provided is to big. The length of the netstring is stored as a `usize`. Should
        /// the message provide a longer value, it is most likely an error and will be returned as such.
        ///
        /// 1. The Separator between length and the netstring is not `b':'`.
        ///
        /// 1. The Netstring does not end with a `b','`.
        fn read_netstring<'a>(&'a mut self, buf: &'a mut [u8]) -> read::ReadMessage<'a, Self>
            where
                Self: Unpin,
        {
            read::read_netstring(self, buf)
        }

        /// This method allows to read one netstring. It returns the netstring as a `Vec<u8>` and
        /// allocates the memory itself, therefore avoiding a to small buffer.
        ///
        /// While this may be use full during development, it should be avoided in production, since it
        /// can allocate memory and a DDOS attack is therefore easily possible.
        ///
        /// # Usage
        /// ```no_exec
        /// use tokio_netstring::NetstringReader;
        ///
        /// let netstring: Vec<u8> = stream.read_netstring_alloc(&mut buf).await.unwrap();
        /// ```
        ///
        /// # Errors
        /// It returns the same errors as [NetstringReader::read_netstring], but can't fail because the
        /// buffer is to small.
        fn read_netstring_alloc(&mut self) -> read_alloc::ReadMessageAlloc<Self>
            where
                Self: Unpin,
        {
            read_alloc::read_netstring_alloc(self)
        }

        /// This method allows to drop one netstring from the stream.
        ///
        /// # Usage
        /// ```no_exec
        /// use tokio_netstring::NetstringReader;
        ///
        /// stream.drop_netstring().await.unwrap();
        /// ```
        ///
        /// # Errors
        /// It returns the same errors as [NetstringReader::read_netstring], but can't fail because the
        /// buffer is to small.
        fn drop_netstring(&mut self) -> drop::DropMessage<Self>
            where
                Self: Unpin,
        {
            drop::drop_netstring(self)
        }
    }

    /// The `NetstringWriter` trait allows to write a slice of bytes as a netstring to any stream that
    /// implements `AsyncWrite`
    pub trait NetstringWriter: AsyncWrite {
        /// Write the slice as a netstring to the stream.
        ///
        /// # Usage
        /// ```no_exec
        /// use tokio_netstring::NetstringWriter;
        ///
        /// let msg = "Hello, World!";
        /// stream.write_netstring(&msg.as_bytes());
        /// ```
        ///
        /// # Errors
        /// This method returns a `tokio::io::Result` which is a re-export from `std::io::Result`. It
        /// returns `ErrorKind::WriteZero` if the stream was closed an no more data can be sent.
        ///
        fn write_netstring<'a>(&'a mut self, buf: &'a [u8]) -> write::WriteMessage<'a, Self>
            where
                Self: Unpin,
        {
            write::write_netstring(self, buf)
        }
    }

    impl<R: AsyncRead + ?Sized> NetstringReader for R {}

    impl<W: AsyncWrite + ?Sized> NetstringWriter for W {}

}
