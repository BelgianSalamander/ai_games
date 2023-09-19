use std::{pin::{pin, Pin}, future::poll_fn, task::{Poll, Context}};

pub struct AsyncReaderWrapper<R: async_std::io::Read + Unpin> {
    reader: R
}

impl<R: async_std::io::Read + Unpin> AsyncReaderWrapper<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader
        }
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        poll_fn(|cx| self.poll_read(cx, buf)).await
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        let mut offset = 0;

        while offset < buf.len() {
            let read = self.read(&mut buf[offset..]).await?;

            offset += read;
        }

        Ok(())
    }

    fn poll_read(&mut self, cx: &mut std::task::Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        let pin = Pin::new(&mut self.reader);

        pin.poll_read(cx, buf)
    }
}


pub struct AsyncWriterWrapper<W: async_std::io::Write + Unpin> {
    writer: W
}

impl<W: async_std::io::Write + Unpin> AsyncWriterWrapper<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer
        }
    }

    pub async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        poll_fn(|cx| self.poll_write(cx, buf)).await
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;

        while offset < buf.len() {
            let written = self.write(&buf[offset..]).await?;

            offset += written;
        }

        Ok(())
    }

    fn poll_write(&mut self, cx: &mut std::task::Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let pin = Pin::new(&mut self.writer);

        pin.poll_write(cx, buf)
    }
}