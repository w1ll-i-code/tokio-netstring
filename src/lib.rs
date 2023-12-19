#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]

//! # NOTICE
//! This is the very first release and my first project in rust. Feedback is appreciated.

use async_trait::async_trait;
use log::trace;
use std::io;
use std::io::{Cursor, ErrorKind, Write};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// The length of a netstring is encoded in decimal. A u32 in decimal is 10 characters long.
// The assumption is made that messages larger than u32::MAX are faulty packages and
// they will therefore not be processed.
const MAX_NETSTRING_LENGTH_DEC: usize = 10;

async fn tag<T: AsyncRead + Unpin + ?Sized>(expected: u8, reader: &mut T) -> io::Result<()> {
    let received = reader.read_u8().await?;
    if expected != received {
        Err(ErrorKind::InvalidData.into())
    } else {
        Ok(())
    }
}

async fn read_netstring_length<T: AsyncRead + Unpin + ?Sized>(reader: &mut T) -> io::Result<usize> {
    let mut buffer = [0u8; MAX_NETSTRING_LENGTH_DEC];
    let mut read_buffer_len = 0usize;

    for i in buffer.iter_mut() {
        match reader.read_u8().await? {
            b':' => break,
            byte @ (b'0'..=b'9') => {
                *i = byte;
                read_buffer_len += 1;
            }
            _ => return Err(ErrorKind::InvalidData.into()),
        }
    }

    if read_buffer_len == MAX_NETSTRING_LENGTH_DEC {
        tag(b':', reader).await?;
    }

    // SAFETY: The validation was already performed when writing into the buffer
    // that this is a valid string that contains only numbers.
    unsafe {
        Ok(std::str::from_utf8_unchecked(&buffer[..read_buffer_len])
            .parse()
            .unwrap())
    }
}

#[cfg(err_drop_message)]
async fn drop_message<T: AsyncRead + Unpin + ?Sized>(
    reader: &mut T,
    mut size: usize,
) -> io::Result<usize> {
    const INTERN_BUFFER_SIZE: usize = 4096;

    let mut intern_buffer = [0u8; INTERN_BUFFER_SIZE];
    while size > INTERN_BUFFER_SIZE {
        size -= reader.read_exact(&mut intern_buffer).await?;
    }
    reader.read_exact(&mut intern_buffer[..size]).await?;

    Err(ErrorKind::BrokenPipe.into())
}

#[cfg(not(err_drop_message))]
async fn drop_message<T: AsyncRead + Unpin + ?Sized>(
    _reader: &mut T,
    _size: usize,
) -> io::Result<usize> {
    Err(ErrorKind::BrokenPipe.into())
}

/// The `AsyncNetstringRead` trait allows you to read one netstring at a time from any stream
/// that has `AsyncRead` implemented. No implementation is thread-safe and multiple simultaneous
/// reads can corrupt the message stream irreparably.
#[async_trait]
pub trait AsyncNetstringRead: AsyncRead + Unpin {
    /// This method allows to read one netstring into the buffer given. It is advised to use
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
    /// Is the feature `err_drop_message` set, then the netstring will be dropped. Therefor is the
    /// stream afterwards in a known stream an can be further used.
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
    ///
    /// In all cases the stream is irreparably corrupted and the connection should therefor be dropped.
    async fn read_netstring(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = read_netstring_length(self).await?;

        if buffer.len() >= length {
            self.read_exact(&mut buffer[..length]).await?;
        } else {
            return drop_message(self, length).await;
        }

        trace!(
            "READING NETSTRING: {}:{},",
            length,
            std::str::from_utf8(&buffer[..length]).unwrap()
        );

        tag(b',', self).await?;

        return Ok(length);
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
    /// It returns the same errors as [AsyncNetstringRead::read_netstring], but can't fail because
    /// the buffer is to small.
    ///
    async fn read_netstring_alloc(&mut self) -> io::Result<Vec<u8>> {
        let length = read_netstring_length(self).await?;
        let mut buffer = Vec::with_capacity(length);

        // SAFETY: The buffer has capacity `length` therefore indexing it until there is safe.
        let buffer_slice = unsafe { buffer.get_unchecked_mut(..length) };

        self.read_exact(buffer_slice).await?;

        tag(b',', self).await?;

        // SAFETY: We have read all the bytes from the source into the Vec. At that point all
        // values up until length have to be initialized.
        unsafe { buffer.set_len(length) };

        return Ok(buffer);
    }
}

impl<Reader: AsyncRead + Unpin + ?Sized> AsyncNetstringRead for Reader {}

/// The `NetstringWriter` trait allows to write a slice of bytes as a netstring to any stream that
/// implements `AsyncWrite`
#[async_trait]
pub trait AsyncNetstringWrite: AsyncWrite + Unpin {
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
    async fn write_netstring(&mut self, data: &[u8]) -> io::Result<()> {
        let mut buffer = [0u8; 2 * MAX_NETSTRING_LENGTH_DEC + 1];
        let len = {
            let mut writer = Cursor::new(&mut buffer[..]);
            write!(writer, "{}", data.len())?;
            writer.position() as usize
        };
        buffer[len] = b':';

        trace!(
            "WRITING NETSTRING: {}{},",
            std::str::from_utf8(&buffer[..len + 1]).unwrap(),
            std::str::from_utf8(data).unwrap()
        );

        self.write_all(&buffer[..len + 1]).await?;
        self.write_all(data).await?;
        self.write_all(b",").await?;
        self.flush().await
    }
}

impl<Writer: AsyncWrite + Unpin + ?Sized> AsyncNetstringWrite for Writer {}
