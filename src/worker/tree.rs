//! Download and unpack repo worktrees into temporary directories.

use std::{io, path::PathBuf};

use bytes::{Buf, Bytes};
use flate2::read::GzDecoder;
use futures::{Stream, StreamExt};
use reqwest::Response;
use tempfile::TempDir;
use tokio::sync::mpsc;

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
        self.rest.advance(self.rest.len() - slice.len());

        result
    }
}

async fn receive_bytes(
    mut stream: impl Stream<Item = reqwest::Result<Bytes>> + Unpin,
    tx: mpsc::Sender<Bytes>,
) -> somehow::Result<()> {
    while let Some(bytes) = stream.next().await {
        tx.send(bytes?).await?;
    }
    Ok(())
}

fn unpack_archive(rx: mpsc::Receiver<Bytes>, path: PathBuf) -> somehow::Result<()> {
    let reader = ReceiverReader::new(rx);
    let reader = GzDecoder::new(reader);
    let mut reader = tar::Archive::new(reader);
    reader.unpack(path)?;
    Ok(())
}

pub async fn download(response: Response) -> somehow::Result<TempDir> {
    let stream = response.error_for_status()?.bytes_stream();

    let dir = TempDir::new()?;
    let path = dir.path().to_path_buf();
    let (tx, rx) = mpsc::channel(1);

    let (received, unpacked) = tokio::join!(
        receive_bytes(stream, tx),
        tokio::task::spawn_blocking(move || unpack_archive(rx, path)),
    );
    received?;
    unpacked??;

    Ok(dir)
}
