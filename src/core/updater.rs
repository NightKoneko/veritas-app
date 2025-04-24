use directories::ProjectDirs;
use reqwest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use self_update;
use tempfile;

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    dll_version: Option<String>,
    version_type: Option<VeritasVersion>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum VeritasVersion {
    GlobalBeta,
    CnBeta,
    GlobalProd,
}

impl Default for VeritasVersion {
    fn default() -> Self {
        Self::GlobalBeta
    }
}

#[derive(Clone)]
pub struct Updater {
    client: reqwest::Client,
    app_version: String,
    project_dirs: ProjectDirs,
    app_release: Option<GithubRelease>,
    dll_release: Option<GithubRelease>,
    pub current_version_type: VeritasVersion,
    checked_dll_version: Option<String>,
}

impl Updater {
    pub fn new() -> Self {
        let project_dirs = ProjectDirs::from("com", "veritas", "veritas-app")
            .expect("Failed to get project directories");

        let version_type = if let Some(config_path) = get_config_path() {
            if let Ok(contents) = fs::read_to_string(config_path) {
                if let Ok(config) = serde_json::from_str::<Config>(&contents) {
                    config.version_type.unwrap_or_default()
                } else {
                    VeritasVersion::default()
                }
            } else {
                VeritasVersion::default()
            }
        } else {
            VeritasVersion::default()
        };
            
        Self {
            client: reqwest::Client::builder()
                .user_agent("veritas-app")
                .build()
                .unwrap(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            project_dirs,
            app_release: None,
            dll_release: None,
            current_version_type: version_type,
            checked_dll_version: None,
        }
    }

    pub fn get_dll_path(&self) -> PathBuf {
        self.project_dirs.data_dir().join("veritas.dll")
    }

    pub async fn check_app_update(&mut self) -> Option<String> {
        match self.client
            .get("https://api.github.com/repos/NightKoneko/veritas-app/releases/latest")
            .send()
            .await 
        {
            Ok(response) => {
                if let Ok(release) = response.json::<GithubRelease>().await {
                    self.app_release = Some(release.clone());
                    
                    if release.tag_name != self_update::cargo_crate_version!() {
                        return Some(release.tag_name);
                    }
                }
            }
            Err(_) => {}
        }
        None
    }

    // TODO: just use self_update instead of this
    pub async fn download_update(&mut self) -> Option<()> {
        let response = self.client
            .get("https://api.github.com/repos/NightKoneko/veritas-app/releases/latest")
            .send()
            .await
            .ok()?
            .json::<GithubRelease>()
            .await
            .ok()?;

        println!("Found release: {}", response.tag_name);
        
        let asset = response.assets.iter()
            .find(|a| a.name.ends_with(".exe"))?;

        println!("Using asset: {}", asset.name);

        let tmp_dir = tempfile::Builder::new()
            .prefix("veritas_update")
            .tempdir()
            .ok()?;

        let exe_bytes = self.client
            .get(&asset.browser_download_url)
            .send()
            .await
            .ok()?
            .bytes()
            .await
            .ok()?;

        let exe_path = tmp_dir.path().join(&asset.name);
        std::fs::write(&exe_path, &exe_bytes).ok()?;

        let current_exe = std::env::current_exe().ok()?;
        let new_exe = current_exe.with_extension("new");
        println!("Moving update to: {:?}", new_exe);
        
        fs::copy(exe_path, &new_exe).ok()?;
        println!("Update ready at: {:?}", new_exe);

        Some(())
    }

    pub async fn check_dll_update(&mut self) -> Option<String> {
        let response = self.client
            .get("https://api.github.com/repos/hessiser/veritas/releases")
            .send()
            .await
            .ok()?;
            
        let releases = response.json::<Vec<GithubRelease>>().await.ok()?;
        
        let version_filter = match self.current_version_type {
            VeritasVersion::GlobalBeta => "global-beta",
            VeritasVersion::CnBeta => "cn-beta",
            VeritasVersion::GlobalProd => "global-prod",
        };

        let release = releases.into_iter()
            .find(|r| r.tag_name.to_lowercase().contains(version_filter))?;

        self.dll_release = Some(release.clone());
        self.checked_dll_version = Some(release.tag_name.clone());

        Some(release.tag_name)
    }

    pub async fn update_dll(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let release = self.dll_release.clone()
            .ok_or("No release info available, please check for updates first")?;

        fs::create_dir_all(self.project_dirs.data_dir())?;

        let dll_asset = release.assets
            .iter()
            .find(|a| a.name == "veritas.dll")
            .ok_or("DLL not found in release")?;

        let dll_bytes = self.client
            .get(&dll_asset.browser_download_url)
            .send()
            .await?
            .bytes()
            .await?;

        let mut file = TokioFile::create(self.get_dll_path()).await?;
        file.write_all(&dll_bytes).await?;

        Ok(())
    }

    pub fn latest_app_version(&self) -> Option<String> {
        self.app_release.as_ref().map(|r| r.tag_name.clone())
    }

    pub fn latest_dll_version(&self) -> Option<String> {
        self.dll_release.as_ref().map(|r| r.tag_name.clone())
    }

    pub fn current_dll_version(&self) -> Option<String> {
        ProjectDirs::from("com", "veritas", "veritas-app")
            .and_then(|proj_dirs| {
                let config_path = proj_dirs.config_dir().join("config.json");
                fs::read_to_string(config_path).ok()
            })
            .and_then(|contents| serde_json::from_str::<Config>(&contents).ok())
            .and_then(|config| config.dll_version)
    }
}

fn get_config_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "veritas", "veritas-app")
        .map(|proj_dirs| proj_dirs.config_dir().join("config.json"))
}
