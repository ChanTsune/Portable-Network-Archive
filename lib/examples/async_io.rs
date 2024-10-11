#![cfg(not(target_family = "wasm"))]
use libpna::{Archive, EntryBuilder, ReadOptions, WriteOptions};
use std::io;
use tokio_util::compat::{
    FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt, TokioAsyncReadCompatExt,
};

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut args = std::env::args();
    let _ = args.next();
    match (args.next().as_deref(), args.next()) {
        (Some("create"), Some(s)) => create(s, &args.collect::<Vec<_>>()).await,
        (Some("extract"), Some(s)) => extract(s).await,
        (f, s) => Err(io::Error::other(format!("{:?}{:?}", f, s))),
    }
}

async fn create(path: String, file_names: &[String]) -> io::Result<()> {
    let file = tokio::fs::File::create(path).await?.compat();
    let mut archive = Archive::write_header_async(file).await?;
    for file_name in file_names {
        let mut file = tokio::fs::File::open(file_name).await?;
        let mut entry_builder =
            EntryBuilder::new_file(file_name.into(), WriteOptions::builder().build())?
                .compat_write();
        tokio::io::copy(&mut file, &mut entry_builder).await?;
        let entry = entry_builder.into_inner().build()?;
        archive.add_entry_async(entry).await?;
    }
    archive.finalize_async().await?;
    Ok(())
}

async fn extract(path: String) -> io::Result<()> {
    let file = tokio::fs::File::open(path).await?.compat();
    let mut archive = Archive::read_header_async(file).await?;
    while let Some(entry) = archive.read_entry_async().await? {
        let mut file = tokio::fs::File::create(entry.header().path().as_path()).await?;
        let mut reader = entry.reader(ReadOptions::builder().build())?.compat();
        tokio::io::copy(&mut reader, &mut file).await?;
    }
    Ok(())
}
