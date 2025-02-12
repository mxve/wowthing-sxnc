use anyhow::Result;
use clap::Parser;
use dialoguer::{Confirm, Input};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[arg(long, help = "API key from wowthing.org")]
    api_key: Option<String>,
    #[arg(long, help = "Path to WoW _retail_ folder")]
    watch_folder: Option<String>,
    #[arg(long, help = "Upload host", default_value = "https://wowthing.org/")]
    upload_host: Option<String>,
    #[arg(long, help = "Upload files on startup")]
    upload_on_startup: Option<bool>,
    #[arg(long, help = "Watch interval in seconds")]
    watch_interval: Option<u16>,
    #[arg(long, help = "Run once and exit")]
    once: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    api_key: String,
    watch_folder: String,
    #[serde(default = "default_upload_host")]
    upload_host: String,
    #[serde(default = "default_true")]
    upload_on_startup: bool,
    #[serde(default = "default_interval")]
    watch_interval: u16,
    #[serde(default)]
    once: bool,
}

fn default_upload_host() -> String {
    "https://wowthing.org/".to_string()
}
fn default_true() -> bool {
    true
}
fn default_interval() -> u16 {
    60
}

impl Settings {
    fn new() -> Self {
        Self {
            api_key: String::new(),
            watch_folder: String::new(),
            upload_host: default_upload_host(),
            upload_on_startup: default_true(),
            watch_interval: default_interval(),
            once: false,
        }
    }

    fn merge_cli(&mut self, cli: &Cli) {
        if let Some(api_key) = &cli.api_key {
            self.api_key = api_key.clone();
        }
        if let Some(watch_folder) = &cli.watch_folder {
            self.watch_folder = watch_folder.clone();
        }
        if let Some(upload_host) = &cli.upload_host {
            self.upload_host = upload_host.clone();
        }
        if let Some(upload_on_startup) = cli.upload_on_startup {
            self.upload_on_startup = upload_on_startup;
        }
        if let Some(watch_interval) = cli.watch_interval {
            self.watch_interval = watch_interval;
        }
        if cli.once {
            self.once = true;
        }
    }

    fn is_valid(&self) -> bool {
        !self.api_key.is_empty() && !self.watch_folder.is_empty()
    }
}

struct FileWatcher {
    watched_paths: Vec<PathBuf>,
    last_updated: HashMap<PathBuf, SystemTime>,
    changed_files: HashMap<PathBuf, SystemTime>,
    client: reqwest::Client,
}

impl FileWatcher {
    fn new() -> Result<Self> {
        Ok(Self {
            watched_paths: Vec::new(),
            last_updated: HashMap::new(),
            changed_files: HashMap::new(),
            client: reqwest::Client::builder()
                .user_agent("WoWthing Sxnc | github.com/mxve/wowthing-sxnc")
                .timeout(Duration::from_secs(20))
                .gzip(true)
                .build()?,
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
            if let Ok(lua_path) = self.find_collector_lua(&sv_path) {
                self.last_updated
                    .insert(lua_path.clone(), fs::metadata(&lua_path)?.modified()?);
                self.watched_paths.push(lua_path);
                println!("Watching {}", entry.path().display());
            }
        }
        Ok(())
    }

    fn find_collector_lua(&self, sv_path: &Path) -> Result<PathBuf> {
        let lua_path = sv_path.join("WoWthing_Collector.lua");
        if lua_path.exists() {
            Ok(lua_path)
        } else {
            Err(anyhow::anyhow!("Collector lua not found"))
        }
    }

    async fn upload_file(&self, file_path: &Path, settings: &Settings) -> Result<()> {
        println!("Uploading {}...", file_path.display());

        let file_content = fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", file_path.display(), e))?;

        let response = self
            .client
            .post(format!("{}api/upload/", settings.upload_host))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "apiKey": settings.api_key,
                "luaFile": file_content,
            }))
            .send()
            .await?;

        if response.status().is_success() {
            println!("Upload successful");
        } else {
            println!(
                "Upload failed: {} - {}",
                response.status(),
                response.text().await?
            );
        }
        Ok(())
    }

    async fn check_for_changes(&mut self, settings: &Settings, force_upload: bool) -> Result<()> {
        let now = SystemTime::now();

        // Check for modified files
        for path in &self.watched_paths {
            if !path.exists() {
                continue;
            }

            let new_mtime = fs::metadata(path)?.modified()?;
            if force_upload || self.last_updated.get(path).map_or(true, |&t| new_mtime > t) {
                self.changed_files.insert(path.clone(), now);
                self.last_updated.insert(path.clone(), new_mtime);
            }
        }

        // Upload changed files after settling period
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

async fn create_config_interactive() -> Result<Settings> {
    println!(
        "No configuration found or missing --api-key and/or --watch-folder. Let's create one!"
    );
    let mut settings = Settings::new();

    settings.api_key = Input::new()
        .with_prompt("Enter your wowthing.org API key")
        .interact()?;
    settings.watch_folder = Input::new()
        .with_prompt("Enter your WoW installation folder path")
        .interact()?;
    settings.upload_host = Input::new()
        .with_prompt("Enter upload host")
        .default(default_upload_host())
        .interact()?;
    settings.upload_on_startup = Confirm::new()
        .with_prompt("Upload files on startup?")
        .default(default_true())
        .interact()?;
    settings.watch_interval = Input::new()
        .with_prompt("Watch interval (seconds)")
        .default(default_interval())
        .interact()?;
    settings.once = Confirm::new()
        .with_prompt("Run once and exit?")
        .default(false)
        .interact()?;

    if let Some(config_path) =
        dirs::config_dir().map(|p| p.join("wowthing-sxnc").join("wowthing-sxnc.toml"))
    {
        if Confirm::new()
            .with_prompt("Save configuration to file?")
            .default(true)
            .interact()?
        {
            fs::create_dir_all(config_path.parent().unwrap())?;
            fs::write(&config_path, toml::to_string_pretty(&settings)?)?;
            println!("Configuration saved to {}", config_path.display());
        }
    }

    Ok(settings)
}

fn load_config() -> Result<Option<Settings>> {
    let config_path =
        dirs::config_dir().map(|p| p.join("wowthing-sxnc").join("wowthing-sxnc.toml"));

    if let Some(path) = config_path {
        if path.exists() {
            return Ok(Some(toml::from_str(&fs::read_to_string(path)?)?));
        }
    }

    if Path::new("wowthing-sxnc.toml").exists() {
        return Ok(Some(toml::from_str(&fs::read_to_string(
            "wowthing-sxnc.toml",
        )?)?));
    }

    Ok(None)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut settings = match load_config()? {
        Some(s) => s,
        None if cli.api_key.is_none() || cli.watch_folder.is_none() => {
            create_config_interactive().await?
        }
        None => Settings::new(),
    };

    settings.merge_cli(&cli);

    if !settings.is_valid() {
        anyhow::bail!("Missing required settings: api_key and watch_folder must be set");
    }

    let mut watcher = FileWatcher::new()?;
    watcher.init_watch_paths(&settings)?;

    if settings.upload_on_startup || settings.once {
        watcher.check_for_changes(&settings, true).await?;
    }

    if settings.once {
        return Ok(());
    }

    loop {
        if let Err(e) = watcher.check_for_changes(&settings, false).await {
            println!("Error checking for changes: {}", e);
        }
        tokio::time::sleep(Duration::from_secs(settings.watch_interval as u64)).await;
    }
}
