use lazy_static::lazy_static;
use std::{
    ffi::OsStr,
    fmt::{self, Display, Formatter},
    fs::{remove_dir_all, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    thread,
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

impl Drop for StoreCfg {
    fn drop(&mut self) {
        if let Some(tx) = &self.compress_tx {
            if let Some(hdl) = self.compress_hdl.take() {
                let _ = tx.send(None);
                let _ = hdl.join();
            }
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
    compress_tx: Option<mpsc::Sender<Option<PathBuf>>>,
    compress_hdl: Option<thread::JoinHandle<()>>,
}

impl StoreCfg {
    pub fn new(root_dir: PathBuf, kind: StoreKind, compress: bool) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(&root_dir)?;
        lazy_static! {
            static ref COMPRESSION_THREAD_TX: Arc<Mutex<Option<mpsc::Sender<Option<PathBuf>>>>> =
                Arc::new(Mutex::new(None));
        }
        // handle compression
        let (compress_tx, compress_hdl) = if compress {
            // if compressing
            if let Ok(mut tx) = COMPRESSION_THREAD_TX.lock() {
                if let Some(tx) = tx.as_ref() {
                    // already initialized
                    (Some(tx.clone()), None) // return the sender
                } else {
                    // we need to initialize the compression thread
                    let (ctx, rx) = mpsc::channel(); // create a channel
                    *tx = Some(ctx.clone()); // store the sender in the mutex
                                             // spawn the compression thread
                    let hdl = thread::spawn(move || {
                        log::info!("Compression thread started");
                        while let Ok(last_dir) = rx.recv() {
                            if let Some(last_dir) = last_dir {
                                // wait for a directory to compress
                                let mut outfile = last_dir.clone(); // create the output file
                                outfile.set_extension("tar.gz");
                                log::info!("Compressing {last_dir:?} to {outfile:?}...");
                                if let Ok(outfile) = File::create(outfile) {
                                    // create the output file
                                    let tar = GzEncoder::new(outfile, Compression::default()); // create the gzip encoder
                                    let mut tar = tar::Builder::new(tar); // create the tar builder
                                    let res = if last_dir.is_dir() {
                                        // if the input is a directory
                                        let root = last_dir.file_name().unwrap_or(OsStr::new(".")); // get the root directory
                                        tar.append_dir_all(root, &last_dir) // append the directory to the tar
                                    } else {
                                        // if the input is a file
                                        tar.append_path(&last_dir) // append the file to the tar
                                    };
                                    match res {
                                        // check the result
                                        Ok(_) => {
                                            // if successful
                                            if let Err(e) = remove_dir_all(&last_dir) {
                                                // delete the input directory
                                                log::warn!(
                                                    "Error deleting directory {last_dir:?}: {e:?}"
                                                );
                                            } else {
                                                log::info!(
                                                    "Compression successful! Deleted {last_dir:?}"
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            // if there was an error
                                            log::warn!("Compression error {e:?}: {last_dir:?}");
                                        }
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                        log::info!("Compression thread exiting");
                    });
                    (Some(ctx), Some(hdl)) // return the sender
                }
            } else {
                (None, None)
            }
        } else {
            // if not compressing
            (None, None)
        };
        Ok(Self {
            root_dir,
            kind,
            current_dir: PathBuf::new(),
            last_date: None,
            last_hour: None,
            writer: None,
            compress_tx,
            compress_hdl,
        })
    }

    pub fn store(&mut self, tstamp: DateTime<Utc>, data: &[u8]) -> Result<(), std::io::Error> {
        let date = tstamp.format("%Y%m%d").to_string();
        let hour = tstamp.format("%H").to_string();
        if self.last_date.as_deref() != Some(&date) {
            // Send the last directory to the compression thread
            if let Some(tx) = &self.compress_tx {
                let _ = tx.send(Some(self.current_dir.clone()));
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

mod test {
    #[test]
    fn test_store() {
        use super::*;
        use chrono::Utc;
        use std::time::Duration;
        use tempfile::tempdir;
        let temp_dir = tempdir().unwrap().into_path();
        let mut st1 = StoreCfg::new(temp_dir.join("test"), StoreKind::Raw, true).unwrap();
        let mut st2 = StoreCfg::new(temp_dir.join("test2"), StoreKind::Json, true).unwrap();
        let data = b"test";
        let tstamp = Utc::now();
        st1.store(tstamp, data).unwrap();
        st2.store(tstamp, data).unwrap();
        thread::sleep(Duration::from_secs(1));
        let tstamp = tstamp + Duration::from_secs(86500); // this will force a compression event
        st1.store(tstamp, data).unwrap();
        st2.store(tstamp, data).unwrap();
    }
}
