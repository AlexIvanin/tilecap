use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            output_dir: default_output_dir(),
        }
    }
}

fn default_output_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Pictures/Screenshots")
}

pub fn load_config(path: Option<&str>) -> Config {
    let config_path = path
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);
    if let Ok(s) = std::fs::read_to_string(&config_path) {
        toml::from_str(&s).unwrap_or_else(|e| {
            eprintln!("tilecap: bad config {config_path:?}: {e}");
            Config::default()
        })
    } else {
        Config::default()
    }
}

fn default_config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".config")
        });
    base.join("tilecap/config.toml")
}
