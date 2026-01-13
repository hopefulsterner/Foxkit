//! Remote filesystem

use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::container::ContainerRuntime;

/// Remote filesystem trait
#[async_trait]
pub trait RemoteFs: Send + Sync {
    /// Read file contents
    async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>>;

    /// Write file contents
    async fn write(&self, path: &Path, content: &[u8]) -> anyhow::Result<()>;

    /// List directory
    async fn list(&self, path: &Path) -> anyhow::Result<Vec<RemoteEntry>>;

    /// Get file/directory info
    async fn stat(&self, path: &Path) -> anyhow::Result<RemoteStat>;

    /// Create directory
    async fn mkdir(&self, path: &Path) -> anyhow::Result<()>;

    /// Remove file
    async fn remove(&self, path: &Path) -> anyhow::Result<()>;

    /// Remove directory
    async fn rmdir(&self, path: &Path, recursive: bool) -> anyhow::Result<()>;

    /// Rename/move
    async fn rename(&self, from: &Path, to: &Path) -> anyhow::Result<()>;

    /// Copy file
    async fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<()>;

    /// Check if path exists
    async fn exists(&self, path: &Path) -> anyhow::Result<bool>;
}

/// Remote directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
}

/// Remote file stat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteStat {
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<u64>,
    pub created: Option<u64>,
    pub permissions: Option<u32>,
}

/// SFTP filesystem
pub struct SftpFs {
    host: String,
    port: u16,
}

impl SftpFs {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }
}

#[async_trait]
impl RemoteFs for SftpFs {
    async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        // Would use SFTP to read
        Ok(Vec::new())
    }

    async fn write(&self, path: &Path, content: &[u8]) -> anyhow::Result<()> {
        Ok(())
    }

    async fn list(&self, path: &Path) -> anyhow::Result<Vec<RemoteEntry>> {
        Ok(Vec::new())
    }

    async fn stat(&self, path: &Path) -> anyhow::Result<RemoteStat> {
        Ok(RemoteStat {
            is_file: true,
            is_dir: false,
            is_symlink: false,
            size: 0,
            modified: None,
            created: None,
            permissions: None,
        })
    }

    async fn mkdir(&self, path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    async fn remove(&self, path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    async fn rmdir(&self, path: &Path, _recursive: bool) -> anyhow::Result<()> {
        Ok(())
    }

    async fn rename(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    async fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    async fn exists(&self, path: &Path) -> anyhow::Result<bool> {
        Ok(false)
    }
}

/// Container filesystem
pub struct ContainerFs {
    container: String,
    runtime: ContainerRuntime,
}

impl ContainerFs {
    pub fn new(container: &str, runtime: ContainerRuntime) -> Self {
        Self {
            container: container.to_string(),
            runtime,
        }
    }

    async fn exec(&self, args: &[&str]) -> anyhow::Result<Vec<u8>> {
        let mut cmd_args = vec!["exec", &self.container];
        cmd_args.extend(args);

        let output = tokio::process::Command::new(self.runtime.command())
            .args(&cmd_args)
            .output()
            .await?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr))
        }
    }
}

#[async_trait]
impl RemoteFs for ContainerFs {
    async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        self.exec(&["cat", &path.to_string_lossy()]).await
    }

    async fn write(&self, path: &Path, content: &[u8]) -> anyhow::Result<()> {
        // Would need to pipe content
        Ok(())
    }

    async fn list(&self, path: &Path) -> anyhow::Result<Vec<RemoteEntry>> {
        let output = self.exec(&["ls", "-la", &path.to_string_lossy()]).await?;
        // Parse ls output
        Ok(Vec::new())
    }

    async fn stat(&self, path: &Path) -> anyhow::Result<RemoteStat> {
        let output = self.exec(&["stat", "-c", "%F %s %Y", &path.to_string_lossy()]).await?;
        // Parse stat output
        Ok(RemoteStat {
            is_file: true,
            is_dir: false,
            is_symlink: false,
            size: 0,
            modified: None,
            created: None,
            permissions: None,
        })
    }

    async fn mkdir(&self, path: &Path) -> anyhow::Result<()> {
        self.exec(&["mkdir", "-p", &path.to_string_lossy()]).await?;
        Ok(())
    }

    async fn remove(&self, path: &Path) -> anyhow::Result<()> {
        self.exec(&["rm", "-f", &path.to_string_lossy()]).await?;
        Ok(())
    }

    async fn rmdir(&self, path: &Path, recursive: bool) -> anyhow::Result<()> {
        if recursive {
            self.exec(&["rm", "-rf", &path.to_string_lossy()]).await?;
        } else {
            self.exec(&["rmdir", &path.to_string_lossy()]).await?;
        }
        Ok(())
    }

    async fn rename(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        self.exec(&["mv", &from.to_string_lossy(), &to.to_string_lossy()]).await?;
        Ok(())
    }

    async fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        self.exec(&["cp", "-r", &from.to_string_lossy(), &to.to_string_lossy()]).await?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> anyhow::Result<bool> {
        match self.exec(&["test", "-e", &path.to_string_lossy()]).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// HTTP-based filesystem
pub struct HttpFs {
    base_url: String,
}

impl HttpFs {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl RemoteFs for HttpFs {
    async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/fs/read", self.base_url))
            .query(&[("path", path.to_string_lossy().to_string())])
            .send()
            .await?;
        
        Ok(response.bytes().await?.to_vec())
    }

    async fn write(&self, path: &Path, content: &[u8]) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/fs/write", self.base_url))
            .query(&[("path", path.to_string_lossy().to_string())])
            .body(content.to_vec())
            .send()
            .await?;
        Ok(())
    }

    async fn list(&self, path: &Path) -> anyhow::Result<Vec<RemoteEntry>> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/fs/list", self.base_url))
            .query(&[("path", path.to_string_lossy().to_string())])
            .send()
            .await?;
        
        Ok(response.json().await?)
    }

    async fn stat(&self, path: &Path) -> anyhow::Result<RemoteStat> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/fs/stat", self.base_url))
            .query(&[("path", path.to_string_lossy().to_string())])
            .send()
            .await?;
        
        Ok(response.json().await?)
    }

    async fn mkdir(&self, path: &Path) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/fs/mkdir", self.base_url))
            .query(&[("path", path.to_string_lossy().to_string())])
            .send()
            .await?;
        Ok(())
    }

    async fn remove(&self, path: &Path) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client
            .delete(format!("{}/fs/file", self.base_url))
            .query(&[("path", path.to_string_lossy().to_string())])
            .send()
            .await?;
        Ok(())
    }

    async fn rmdir(&self, path: &Path, recursive: bool) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client
            .delete(format!("{}/fs/dir", self.base_url))
            .query(&[
                ("path", path.to_string_lossy().to_string()),
                ("recursive", recursive.to_string()),
            ])
            .send()
            .await?;
        Ok(())
    }

    async fn rename(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/fs/rename", self.base_url))
            .json(&serde_json::json!({
                "from": from.to_string_lossy(),
                "to": to.to_string_lossy(),
            }))
            .send()
            .await?;
        Ok(())
    }

    async fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        client
            .post(format!("{}/fs/copy", self.base_url))
            .json(&serde_json::json!({
                "from": from.to_string_lossy(),
                "to": to.to_string_lossy(),
            }))
            .send()
            .await?;
        Ok(())
    }

    async fn exists(&self, path: &Path) -> anyhow::Result<bool> {
        match self.stat(path).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
