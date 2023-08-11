//! Stream gzipped tar-ed repository worktrees.

use std::{
    io::{self, BufWriter, Read},
    sync::Arc,
};

use axum::{body::Bytes, BoxError};
use flate2::{write::GzEncoder, Compression};
use futures::TryStream;
use gix::{
    bstr::ByteSlice, objs::tree::EntryMode, prelude::ObjectIdExt, worktree::stream::Entry,
    ObjectId, ThreadSafeRepository,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, warn};

const BLOCK_SIZE: usize = 1024 * 1024;
const COMPRESSION_LEVEL: Compression = Compression::fast();

struct SenderWriter(mpsc::Sender<Result<Bytes, BoxError>>);

impl SenderWriter {
    fn new(tx: mpsc::Sender<Result<Bytes, BoxError>>) -> Self {
        Self(tx)
    }

    fn finish(self) {}
}

impl io::Write for SenderWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.0.blocking_send(Ok(Bytes::copy_from_slice(buf))) {
            Ok(()) => Ok(buf.len()),
            Err(_) => Err(io::ErrorKind::ConnectionAborted.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_entry(
    mut entry: Entry<'_>,
    writer: &mut tar::Builder<impl io::Write>,
) -> Result<(), BoxError> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(match entry.mode {
        EntryMode::Tree | EntryMode::Commit => tar::EntryType::Directory,
        EntryMode::Blob | EntryMode::BlobExecutable => tar::EntryType::Regular,
        EntryMode::Link => tar::EntryType::Symlink,
    });
    header.set_mode(match entry.mode {
        EntryMode::BlobExecutable => 0o755, // rwxr-xr-x
        _ => 0o644,                         // rw-r--r--
    });

    if entry.mode == EntryMode::Link {
        let mut buf = vec![];
        entry.read_to_end(&mut buf)?;
        header.set_size(0);
        let path = gix::path::from_bstr(entry.relative_path());
        let target = gix::path::from_bstr(buf.as_bstr());
        writer.append_link(&mut header, path, target)?;
    } else {
        header.set_size(entry.bytes_remaining().unwrap_or(0) as u64);
        let path = gix::path::from_bstr(entry.relative_path()).to_path_buf();
        writer.append_data(&mut header, path, entry)?;
    }

    Ok(())
}

fn write_worktree(
    repo: Arc<ThreadSafeRepository>,
    commit_id: ObjectId,
    tx: mpsc::Sender<Result<Bytes, BoxError>>,
) -> Result<(), BoxError> {
    let repo = repo.to_thread_local();
    let tree_id = commit_id
        .attach(&repo)
        .object()?
        .try_into_commit()?
        .tree_id()?;
    let (mut stream, _) = repo.worktree_stream(tree_id)?;

    let writer = SenderWriter::new(tx);
    let writer = BufWriter::with_capacity(BLOCK_SIZE, writer);
    let writer = GzEncoder::new(writer, COMPRESSION_LEVEL);
    let mut writer = tar::Builder::new(writer);

    while let Some(entry) = stream.next_entry()? {
        write_entry(entry, &mut writer)?;
    }

    writer
        .into_inner()?
        .finish()?
        .into_inner()
        .map_err(|e| e.into_error())?
        .finish();

    Ok(())
}

pub fn tar_and_gzip(
    repo: Arc<ThreadSafeRepository>,
    id: ObjectId,
) -> impl TryStream<Ok = Bytes, Error = BoxError> {
    let (tx, rx) = mpsc::channel(1);
    tokio::task::spawn_blocking(move || {
        if let Err(e) = write_worktree(repo, id, tx.clone()) {
            warn!("Error while streaming tar:\n{e:?}");
            let _ = tx.blocking_send(Err(e));
        } else {
            debug!("Tar streamed successfully");
        }
    });
    ReceiverStream::new(rx)
}
