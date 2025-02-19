use std::{
    ffi::OsStr, fmt::{self, Display, Formatter}, fs::{remove_dir_all, File, OpenOptions}, io::Write, path::PathBuf, thread
};

use chrono::{DateTime, Utc};
use flate2::{write::GzEncoder, Compression};
use ublox_gps_tec::DEFAULT_DELIM;

#[derive(Debug)]
pub enum StoreKind {
    Raw,
    Json,
}

impl StoreKind {
    fn delimiter(&self) -> &'static [u8] {
        match self {
            StoreKind::Raw => &DEFAULT_DELIM,
            StoreKind::Json => b"\n",
        }
    }
}

impl Display for StoreKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            StoreKind::Raw => write!(f, "bin"),
            StoreKind::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug)]
pub struct StoreCfg {
    root_dir: PathBuf,
    kind: StoreKind,
    current_dir: PathBuf,
    last_date: Option<String>,
    last_hour: Option<String>,
    writer: Option<File>,
    compress: bool,
}

impl StoreCfg {
    pub fn new(root_dir: PathBuf, kind: StoreKind) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(&root_dir)?;
        Ok(Self {
            root_dir,
            kind,
            current_dir: PathBuf::new(),
            last_date: None,
            last_hour: None,
            writer: None,
            compress: false,
        })
    }

    pub fn set_compression(&mut self, comp: bool) {
        self.compress = comp;
    }
    
    pub fn store(&mut self, tstamp: DateTime<Utc>, data: &[u8]) -> Result<(), std::io::Error> {
        let date = tstamp.format("%Y%m%d").to_string();
        let hour = tstamp.format("%H").to_string();
        if self.last_date.as_deref() != Some(&date) {
            if self.compress {
                let last_dir = self.current_dir.clone();
                let _ = thread::spawn(
                    move || {
                        let mut outfile = last_dir.clone();
                        outfile.set_extension("tar.gz");
                        if let Ok(outfile) = File::create(outfile) {
                            let tar = GzEncoder::new(outfile, Compression::default());
                            let mut tar = tar::Builder::new(tar);
                            let res = if last_dir.is_dir() {
                                let root = last_dir.file_name().unwrap_or(OsStr::new("."));
                                tar.append_dir_all(root, &last_dir)
                            } else {
                                tar.append_path(&last_dir)
                            };
                            match res {
                                Ok(_) => {
                                    if let Err(e) = remove_dir_all(&last_dir) {
                                        println!("Error deleting directory {last_dir:?}: {e:?}");
                                    }
                                }
                                Err(e) => {
                                    println!("Compression error {e:?}: {last_dir:?}");
                                } 
                            }
                        }
                    }
                );
            }
            self.current_dir = self.root_dir.join(&date);
            std::fs::create_dir_all(&self.current_dir)?;
            self.last_date = Some(date.clone());
            self.last_hour = None;
        }
        if self.last_hour.as_deref() != Some(&hour) {
            let filename = self
                .current_dir
                .join(format!("{}{}0000.{}", &date, &hour, self.kind));
            if filename.exists() {
                self.writer = Some(OpenOptions::new().append(true).open(filename)?);
            } else {
                self.writer = Some(File::create(filename)?);
            }
            self.last_hour = Some(hour);
        }
        if let Some(writer) = &mut self.writer {
            writer.write_all(data)?;
            writer.write_all(self.kind.delimiter())?;
            writer.flush()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No file writer",
            ))
        }
    }
}
