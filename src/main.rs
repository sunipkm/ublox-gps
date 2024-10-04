use std::{
    io::{ErrorKind, Write},
    time::{Duration, Instant},
};

use nmea::{GpsError, NmeaGpsInfo, RawNmea};
use nmea_parser::{NmeaParser, ParsedMessage};
use ubx::{split_ubx, UbxFormat, UbxGpsInfo, UbxRxmRawx};

pub mod ubx;
pub mod nmea;

fn main() {
    let mut serial_port = serialport::new("/dev/ttyUSB0", 115200)
        .open()
        .expect("Failed to open serial port");
    serial_port
        .set_timeout(Duration::from_secs_f32(0.1))
        .expect("Failed to set timeout");
    let mut ofile = std::fs::File::create("gps_data.txt").expect("Failed to create file");
    let mut now = Instant::now();
    loop {
        let last = now;
        let mut buf = Vec::with_capacity(1024);
        if let Err(err) = serial_port.read_to_end(&mut buf) {
            if err.kind() != ErrorKind::TimedOut {
                eprintln!("Failed to read from serial port: {}", err);
                break;
            }
        }
        if buf.is_empty() {
            continue;
        }
        if ofile.write_all(&buf).is_err() {
            println!("Failed to write to file");
        }
        println!("----------------------------------------");
        print!("Read: {} bytes\t", buf.len());
        now = Instant::now();
        println!("Time: {} s", (now - last).as_secs_f32());
        let ubxinfo = parse_messages(buf);
        match ubxinfo {
            Ok(ubxinfo) => {
                let (mfcount, mfsats) = ubxinfo.carrier_phase().into_iter().fold((0, Vec::new()), |mut count, (sat, ch)| {
                    if ch.meas.len() > 1 {
                        count.0 += 1;
                        count.1.push(sat);
                    }
                    count
                });
                println!("Timestamp: {:?}, Location: {:?}, Multi-frequency: {}/{}, Satellites: {:?}", ubxinfo.timestamp(), ubxinfo.location(),  mfcount, ubxinfo.carrier_phase().len(), mfsats);
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
            }
        }
        println!("++++++++++++++++++++++++++++++++++++++++");
        // println!("{:?}", buf);
    }
}

fn parse_messages(buf: Vec<u8>) -> Result<UbxGpsInfo, GpsError> {
    // 1. Separate into UBX and NMEA messages
    let (ubx, buf) = split_ubx(buf);
    // 2. Parse UBX messages
    let mut rxm = Vec::new();
    for msg in ubx {
        if let Ok(msg) = UbxRxmRawx::from_message(msg) {
            rxm.push(msg);
        }
    }
    // 3. Parse NMEA messages
    let buf = std::str::from_utf8(&buf).map_err(|e| GpsError::ParseError(e.to_string()))?;
    let gpsmsg = RawNmea::parse_str(buf);
    let gpsmsg = NmeaGpsInfo::create(&gpsmsg)?;
    let gpsinfo = UbxGpsInfo::new(gpsmsg, rxm.pop());
    Ok(gpsinfo)
}
