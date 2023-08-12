//! Download and unpack repo worktrees into temporary directories.

use std::{io, path::PathBuf};

use axum::body::Bytes;
use flate2::read::GzDecoder;
use futures::{Stream, StreamExt};
use tempfile::TempDir;
use tokio::{select, sync::mpsc};
use tracing::debug;

use crate::somehow;

struct ReceiverReader {
    rx: mpsc::Receiver<Bytes>,
    rest: Bytes,
}

impl ReceiverReader {
    fn new(rx: mpsc::Receiver<Bytes>) -> Self {
        Self {
            rx,
            rest: Bytes::new(),
        }
    }
}

impl io::Read for ReceiverReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.rest.is_empty() {
            if let Some(bytes) = self.rx.blocking_recv() {
                self.rest = bytes;
            }
        }

        let mut slice = &*self.rest;
        let result = slice.read(buf);
        let _ = self.rest.split_to(self.rest.len() - slice.len());

        result
    }
}

pub struct UnpackedTree {
    pub hash: String,
    pub dir: TempDir,
}

impl UnpackedTree {
    async fn stream(
        mut stream: impl Stream<Item = reqwest::Result<Bytes>> + Unpin,
        tx: mpsc::Sender<Bytes>,
    ) -> somehow::Result<()> {
        while let Some(bytes) = stream.next().await {
            tx.send(bytes?).await?;
        }
        Ok(())
    }

    fn unpack(rx: mpsc::Receiver<Bytes>, path: PathBuf) -> somehow::Result<()> {
        let reader = ReceiverReader::new(rx);
        let reader = GzDecoder::new(reader);
        let mut reader = tar::Archive::new(reader);
        reader.unpack(path)?;
        Ok(())
    }

    pub async fn download(url: &str, hash: String) -> somehow::Result<Self> {
        let dir = TempDir::new()?;
        debug!(
            "Downloading and unpacking {url} to {}",
            dir.path().display()
        );
        let (tx, rx) = mpsc::channel(1);
        let stream = reqwest::get(url).await?.bytes_stream();

        let path = dir.path().to_path_buf();
        let unpack_task = tokio::task::spawn_blocking(move || Self::unpack(rx, path));

        select! {
            r = Self::stream(stream, tx) => r?,
            r = unpack_task => r??,
        }

        Ok(Self { hash, dir })
    }
}
