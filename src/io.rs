use std::path::Path;

use anyhow::Result;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
};

pub async fn save_file<P>(name: P, bytes: &[u8]) -> Result<()>
where
    P: AsRef<Path>,
{
    create_dir_all(name.as_ref().parent().unwrap()).await?;
    let mut file = File::create(name).await?;
    file.write_all(bytes).await?;
    Ok(())
}
