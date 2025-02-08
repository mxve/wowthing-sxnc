use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Debug, Serialize)]
struct ApiUpload {
    #[serde(rename = "apiKey")]
    api_key: String,
    #[serde(rename = "luaFile")]
    lua_file: String,
}

#[derive(Debug, Deserialize)]
struct Settings {
    api_key: String,
    watch_folder: String,
    upload_host: String,
    upload_on_startup: bool,
    watch_interval: u16,
}

struct FileWatcher {
    watched_paths: Vec<PathBuf>,
    last_updated: HashMap<PathBuf, SystemTime>,
    changed_files: HashMap<PathBuf, SystemTime>,
    client: reqwest::Client,
}

impl FileWatcher {
    fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("WoWthing Sxnc | github.com/mxve/wowthing-sxnc")
            .timeout(Duration::from_secs(20))
            .gzip(true)
            .build()?;

        Ok(Self {
            watched_paths: Vec::new(),
            last_updated: HashMap::new(),
            changed_files: HashMap::new(),
            client,
        })
    }

    fn init_watch_paths(&mut self, settings: &Settings) -> Result<()> {
        let wtf_path = Path::new(&settings.watch_folder).join("WTF/Account");
        if !wtf_path.exists() {
            println!("ERROR! Path does not exist: {}", wtf_path.display());
            return Ok(());
        }

        for entry in fs::read_dir(wtf_path)? {
            let entry = entry?;
            let sv_path = entry.path().join("SavedVariables");
            if sv_path.exists() {
                let lua_path = sv_path.join("WoWthing_Collector.lua");
                if lua_path.exists() {
                    self.last_updated
                        .insert(lua_path.clone(), fs::metadata(&lua_path)?.modified()?);
                    self.watched_paths.push(lua_path);
                    println!("Watching {}", entry.path().display());
                }
            }
        }
        Ok(())
    }

    async fn upload_file(&self, file_path: &Path, settings: &Settings) -> Result<()> {
        println!("Uploading {}...", file_path.display());

        let file_content = fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file_path.display(), e))?;

        let upload = ApiUpload {
            api_key: settings.api_key.clone(),
            lua_file: file_content,
        };

        let response = self
            .client
            .post(format!("{}api/upload/", settings.upload_host))
            .header("Content-Type", "application/json")
            .json(&upload)
            .send()
            .await?;

        match response.status().is_success() {
            true => println!("Upload successful."),
            false => {
                let status = response.status();
                let error = response.text().await?;
                println!("Upload failed: {} - {}", status, error);
            }
        }
        Ok(())
    }

    async fn check_for_changes(&mut self, settings: &Settings, force_upload: bool) -> Result<()> {
        let now = SystemTime::now();

        for path in &self.watched_paths {
            if !path.exists() {
                continue;
            }

            let new_mtime = fs::metadata(path)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to get metadata for {}: {}", path.display(), e)
                })?
                .modified()?;

            if force_upload
                || self
                    .last_updated
                    .get(path)
                    .map_or(true, |&old_mtime| new_mtime > old_mtime)
            {
                self.changed_files.insert(path.clone(), now);
                self.last_updated.insert(path.clone(), new_mtime);
            }
        }

        let files: Vec<_> = self
            .changed_files
            .iter()
            .filter(|(_, &time)| {
                force_upload
                    || now.duration_since(time).unwrap_or_default() > Duration::from_secs(2)
            })
            .map(|(path, _)| path.clone())
            .collect();

        for path in files {
            self.changed_files.remove(&path);
            if let Err(e) = self.upload_file(&path, settings).await {
                println!("Failed to upload {}: {}", path.display(), e);
            }
        }
        Ok(())
    }
}

fn load_config() -> Result<Settings> {
    let config_builder = config::Config::builder();

    let config_builder = if let Some(config_dir) = dirs::config_dir() {
        let config_path = config_dir.join("wowthing-sxnc").join("wowthing-sxnc.toml");
        if config_path.exists() {
            config_builder.add_source(config::File::from(config_path))
        } else {
            config_builder.add_source(config::File::with_name("wowthing-sxnc"))
        }
    } else {
        config_builder.add_source(config::File::with_name("wowthing-sxnc"))
    };

    Ok(config_builder.build()?.try_deserialize()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let settings = load_config()?;

    let mut watcher = FileWatcher::new()?;
    watcher.init_watch_paths(&settings)?;

    if settings.upload_on_startup {
        watcher.check_for_changes(&settings, true).await?;
    }

    loop {
        if let Err(e) = watcher.check_for_changes(&settings, false).await {
            println!("Error checking for changes: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(settings.watch_interval as u64)).await;
    }
}
