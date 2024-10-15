use std::path::PathBuf;

use argh::FromArgs;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(FromArgs, Serialize, Deserialize, Debug)]
/// Configuration for the recorder
pub struct RecorderCfg {
    /// serial device
    #[argh(positional)]
    pub serial_port: String,
    /// baud rate
    #[argh(option, default = "115200")]
    pub baud_rate: u32,
    /// timeout in milliseconds
    #[argh(option, default = "100")]
    pub timeout: u64,
    /// save data to this directory
    #[argh(option, default = "PathBuf::from(\".\")")]
    pub save_dir: PathBuf,
}

impl RecorderCfg {
    /// Store the configuration in the default location
    pub fn store_default(&self) -> Result<(), std::io::Error> {
        let mut path = get_default_path();
        std::fs::create_dir_all(&path)?;
        path.push("config.json");
        std::fs::write(
            path,
            serde_json::to_string(self)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        )
    }

    /// Load the configuration from the default location
    pub fn load_default() -> Result<Self, std::io::Error> {
        let mut path = get_default_path();
        path.push("config.json");
        let data = std::fs::read(path)?;
        serde_json::from_slice(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

fn get_default_path() -> PathBuf {
    if let Some(path) = ProjectDirs::from("", "", "ublox_gps_recorder") {
        path.config_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}
