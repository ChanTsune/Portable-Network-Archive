use crate::{Chunk, ChunkType, RawChunk};
use futures_io::AsyncRead;
use futures_util::Stream;
use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
};

/// Returns a stream over the chunks that follow the archive signature.
///
/// The reader must be positioned at the first chunk's length field. This
/// function does not read, skip, or validate the archive signature; call
/// [`read_signature`](crate::async_io::read_signature) first. Archive-level
/// chunk ordering is not validated.
///
/// `max_data_len` is an inclusive upper bound for each chunk's data length.
/// Pass [`u32::MAX`] to allow the full range representable by the PNA format.
///
/// The stream includes the archive-end ([`ChunkType::AEND`]) chunk and
/// returns `None` after yielding it. If parsing a chunk fails, the error is
/// yielded once and the stream then returns `None`. Each chunk has the error
/// semantics of [`read_chunk`](crate::async_io::read_chunk), subject to
/// `max_data_len`.
#[inline]
pub fn read_chunks<'a, R: AsyncRead + Unpin + Send + ?Sized>(
    reader: &'a mut R,
    max_data_len: u32,
) -> impl Stream<Item = io::Result<RawChunk<Vec<u8>>>> + 'a {
    #[allow(clippy::type_complexity)]
    struct Chunks<'a, R: AsyncRead + Unpin + Send + ?Sized> {
        reader: Option<&'a mut R>,
        max_data_len: u32,
        done: bool,
        fut: Option<
            Pin<Box<dyn Future<Output = (&'a mut R, io::Result<RawChunk<Vec<u8>>>)> + Send + 'a>>,
        >,
    }

    impl<'a, R: AsyncRead + Unpin + Send + ?Sized> Stream for Chunks<'a, R> {
        type Item = io::Result<RawChunk<Vec<u8>>>;

        #[inline]
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = self.get_mut();
            if this.fut.is_none() {
                if this.done {
                    return Poll::Ready(None);
                }
                let reader = this
                    .reader
                    .take()
                    .expect("reader is present while no chunk read is in flight");
                let max_data_len = this.max_data_len;
                this.fut = Some(Box::pin(async move {
                    let result = crate::async_io::read_chunk(&mut *reader, max_data_len).await;
                    (reader, result)
                }));
            }
            let outcome = this
                .fut
                .as_mut()
                .expect("chunk read future was just created or already in flight")
                .as_mut()
                .poll(cx);
            match outcome {
                Poll::Pending => Poll::Pending,
                Poll::Ready((reader, result)) => {
                    this.fut = None;
                    this.reader = Some(reader);
                    match result {
                        Ok(chunk) => {
                            this.done = chunk.ty() == ChunkType::AEND;
                            Poll::Ready(Some(Ok(chunk)))
                        }
                        Err(error) => {
                            this.done = true;
                            Poll::Ready(Some(Err(error)))
                        }
                    }
                }
            }
        }
    }

    Chunks {
        reader: Some(reader),
        max_data_len,
        done: false,
        fut: None,
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use crate::async_io::read_signature;
    use crate::chunk::test_support::{raw_chunk_bytes, valid_chunk_bytes};
    use futures_util::{AsyncReadExt, TryStreamExt, io::Cursor};

    fn assert_send<T: Send>(stream: T) -> T {
        stream
    }

    #[tokio::test]
    async fn read_chunks_starts_at_the_current_position() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut reader = Cursor::new(input);
        let mut chunks = assert_send(read_chunks(&mut reader, u32::MAX));

        let mut types = Vec::new();
        while let Some(chunk) = chunks.try_next().await.unwrap() {
            types.push(chunk.ty());
        }

        assert_eq!(types, [ChunkType::FDAT, ChunkType::AEND]);
    }

    #[tokio::test]
    async fn read_chunks_stops_after_aend() {
        let mut input = raw_chunk_bytes(*b"AEND", &[]);
        input.extend_from_slice(&valid_chunk_bytes());

        let mut reader = Cursor::new(input);
        let mut chunks = read_chunks(&mut reader, u32::MAX);

        let mut types = Vec::new();
        while let Some(chunk) = chunks.try_next().await.unwrap() {
            types.push(chunk.ty());
        }

        assert_eq!(types, [ChunkType::AEND]);
    }

    #[tokio::test]
    async fn read_chunks_yields_the_first_error_once_and_stops() {
        let mut input = valid_chunk_bytes();
        *input.last_mut().unwrap() ^= 0xFF;
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut reader = Cursor::new(input);
        let mut chunks = read_chunks(&mut reader, u32::MAX);

        assert!(chunks.try_next().await.is_err());
        assert!(chunks.try_next().await.unwrap().is_none());
        assert!(chunks.try_next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn read_chunks_reads_an_archive_after_its_signature() {
        let archive = include_bytes!("../../../resources/test/zstd.pna");
        let mut reader = Cursor::new(archive);
        read_signature(&mut reader).await.unwrap();

        let mut chunks = read_chunks(&mut reader, u32::MAX);

        let mut types = Vec::new();
        while let Some(chunk) = chunks.try_next().await.unwrap() {
            types.push(chunk.ty());
        }

        assert_eq!(types.first(), Some(&ChunkType::AHED));
        assert_eq!(types.last(), Some(&ChunkType::AEND));
    }

    struct PendingReader<R> {
        inner: R,
        pending_next: bool,
    }

    impl<R: AsyncRead + Unpin> AsyncRead for PendingReader<R> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            if this.pending_next {
                this.pending_next = false;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            this.pending_next = true;
            Pin::new(&mut this.inner).poll_read(cx, buf)
        }
    }

    #[tokio::test]
    async fn read_chunks_resumes_after_pending() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut reader = PendingReader {
            inner: Cursor::new(input),
            pending_next: true,
        };
        let mut chunks = read_chunks(&mut reader, u32::MAX);

        let mut types = Vec::new();
        while let Some(chunk) = chunks.try_next().await.unwrap() {
            types.push(chunk.ty());
        }

        assert_eq!(types, [ChunkType::FDAT, ChunkType::AEND]);
        assert!(chunks.try_next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn read_chunks_returns_none_after_termination() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut reader = Cursor::new(input);
        let mut chunks = read_chunks(&mut reader, u32::MAX);

        while chunks.try_next().await.unwrap().is_some() {}

        assert!(chunks.try_next().await.unwrap().is_none());
        assert!(chunks.try_next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn read_chunks_leaves_the_reader_after_aend() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));
        input.extend_from_slice(b"tail");

        let mut reader = Cursor::new(input);

        let types = {
            let mut chunks = read_chunks(&mut reader, u32::MAX);
            let mut types = Vec::new();
            while let Some(chunk) = chunks.try_next().await.unwrap() {
                types.push(chunk.ty());
            }
            types
        };

        assert_eq!(types, [ChunkType::FDAT, ChunkType::AEND]);

        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"tail");
    }

    #[tokio::test]
    async fn read_chunks_yields_unexpected_eof_once_for_empty_input() {
        let mut reader = Cursor::new(Vec::new());
        let mut chunks = read_chunks(&mut reader, u32::MAX);

        assert_eq!(
            chunks.try_next().await.err().unwrap().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(chunks.try_next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn read_chunks_enforces_max_data_len() {
        let mut reader = Cursor::new(valid_chunk_bytes());
        let mut chunks = read_chunks(&mut reader, 0);

        assert_eq!(
            chunks.try_next().await.err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(chunks.try_next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn read_chunks_accepts_a_dyn_reader() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));

        let mut cursor = Cursor::new(input);
        let reader: &mut (dyn AsyncRead + Send + Unpin) = &mut cursor;
        let mut chunks = read_chunks(reader, u32::MAX);

        let mut types = Vec::new();
        while let Some(chunk) = chunks.try_next().await.unwrap() {
            types.push(chunk.ty());
        }

        assert_eq!(types, [ChunkType::FDAT, ChunkType::AEND]);
    }
}
