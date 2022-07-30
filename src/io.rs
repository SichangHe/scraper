use std::path::Path;

use anyhow::Result;
use tokio::{
    fs::{create_dir_all, File, OpenOptions},
    io::AsyncWriteExt,
    spawn,
    task::JoinHandle,
};

async fn create_parent_dirs_for(path: &Path) -> Result<()> {
    create_dir_all(path.parent().unwrap()).await?;
    Ok(())
}

pub async fn create_file<P>(name: P) -> Result<File>
where
    P: AsRef<Path>,
{
    create_parent_dirs_for(name.as_ref()).await?;
    let file = File::create(name).await?;
    Ok(file)
}

pub async fn save_file<P, B>(name: P, bytes: B) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<[u8]>,
{
    let mut file = create_file(name).await?;
    file.write_all(bytes.as_ref()).await?;
    Ok(())
}

pub async fn append_file<P>(name: P) -> Result<File>
where
    P: AsRef<Path>,
{
    create_parent_dirs_for(name.as_ref()).await?;
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&name)
        .await?;
    Ok(file)
}

pub async fn append_to_file<P>(name: P, bytes: &[u8]) -> Result<()>
where
    P: AsRef<Path>,
{
    let mut file = append_file(name).await?;
    file.write_all(bytes).await?;
    Ok(())
}

#[derive(Debug)]
pub struct Writer {
    pub handle: JoinHandle<Result<()>>,
}

impl Writer {
    pub async fn spawn<P, B>(name: P, bytes: B) -> Self
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        let name = name.as_ref().to_owned();
        let bytes = bytes.as_ref().to_owned();
        Self {
            handle: spawn(async {
                save_file(name, bytes).await?;
                Ok(())
            }),
        }
    }

    pub async fn wait(self) -> Result<()> {
        self.handle.await?
    }
}
