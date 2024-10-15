#![deny(missing_docs)]
//! # Recorder
mod config;
mod store;
use chrono::Utc;
use crossterm::terminal;
use ublox_gps_tec::{GnssFreq, GnssSatellite};
use std::{io::ErrorKind, path::{Path, PathBuf}, time::Duration};

pub use config::RecorderCfg;
use serialport::SerialPort;
use store::{StoreCfg, StoreKind};

fn main() {
    // Try to load the config file and open the serial port from the config file
    let save_dir = Path::new("./");
    let mut ser = serialport::new("/dev/ttyUSB0", 115200).open().expect("Failed to open serial port");
    // Set the timeout on the serial port
    ser.set_timeout(Duration::from_millis(100))
        .expect("Failed to set timeout");
    // Create the raw data directory
    let raw_dir = save_dir.join("raw");
    let mut raw_writer =
        StoreCfg::new(raw_dir, StoreKind::Raw).expect("Failed to create raw data directory");
    // Create the TEC data directory
    let tec_dir = save_dir.join("tec");
    let mut tec_writer =
        StoreCfg::new(tec_dir, StoreKind::Json).expect("Failed to create TEC data directory");
    {}
    loop {
        let systime = Utc::now();
        let mut buf = Vec::with_capacity(4096);
        if let Err(err) = ser.read_to_end(&mut buf) {
            if err.kind() != ErrorKind::TimedOut {
                eprintln!("Error reading from serial port: {}", err);
                break;
            }
        }
        if buf.is_empty() {
            continue;
        }
        raw_writer
            .store(systime, &buf)
            .expect("Failed to store raw data");
        let ubxinfo = ublox_gps_tec::parse_messages(buf);
        match ubxinfo {
            Ok(info) => {
                if let Some(tec) = ublox_gps_tec::TecInfo::assimilate(&info) {
                    tec_writer
                        .store(
                            tec.timestamp(),
                            serde_json::to_string(&tec)
                                .expect("Could not convert TEC data to JSON string")
                                .as_bytes(),
                        )
                        .expect("Failed to store TEC data");
                    let width = terminal::size().expect("Failed to get terminal size").0;
                    // header
                    println!(
                        "\n\n{:-<width$}",
                        format!(
                            "{} ({:.3}, {:.3}, {:.3}) ",
                            tec.timestamp().format("%Y-%m-%d %H:%M:%S%Z"),
                            tec.location().0,
                            tec.location().1,
                            tec.location().2 * 1e-3
                        ),
                        width = width as usize
                    );
                    // TEC data
                    for tinfo in tec.tec() {
                        let meas = &info.carrier_phase()[&tinfo.source()]; // safe unwrap
                        let src = match tinfo.source() {
                            GnssSatellite::Gps(prn) => format!("GPS-{:>2}", prn),
                            GnssSatellite::Galileo(prn) => format!("GAL-{:>2}", prn),
                            GnssSatellite::Beidou(prn) => format!("BEI-{:>2}", prn),
                            GnssSatellite::Glonass(prn) => format!("GLO-{:>2}", prn),
                            GnssSatellite::Qzss(prn) => format!("QZS-{:>2}", prn),
                            GnssSatellite::Sbas(prn) => format!("SBA-{:>2}", prn),
                        };
                        print!(
                            "\t{}: {:>3} AZ {:>2} EL | ",
                            src,
                            tinfo.azimuth(),
                            tinfo.elevation()
                        );
                        if let Some(ptec) = tinfo.phase_tec() {
                            print!("Phase: {:.3} +/- {:.3} | ", ptec.0, ptec.1);
                        } else {
                            print!("Phase: N/A | ");
                        }
                        if let Some(rtec) = tinfo.range_tec() {
                            print!("Range: {:.3} +/- {:.3} | ", rtec.0, rtec.1);
                        } else {
                            print!("Range: N/A | ");
                        }
                        for m in &meas.meas {
                            use GnssFreq::*;
                            match m.channel {
                                Gps(freq) => print!("{:?}: ", freq),
                                Galileo(freq) => print!("{:?}: ", freq),
                                Beidou(freq) => print!("{:?}: ", freq),
                                Glonass(freq) => print!("{:?}: ", freq),
                                Qzss(freq) => print!("{:?}: ", freq),
                            }
                            if let Some(prn) = m.pseudo_range {
                                print!("PRN {:.3}±{:.3} km, ", prn.0*1e-3, prn.1*1e-3);
                            } else {
                                print!("PRN N/A, ");
                            }
                            if let Some(cp) = m.carrier_phase {
                                print!("CP {:.3}±{:.3} kHz, ", cp.0*1e-3, cp.1*1e-3);
                            } else {
                                print!("CP N/A, ");
                            }
                        }
                        println!();
                    }
                    println!("{:=<width$}", "", width = width as usize);
                } else {
                    eprintln!("Source did not contain information for TEC calculation");
                }
            }
            Err(e) => eprintln!("Error parsing UBX messages: {}", e),
        }
    }
}
