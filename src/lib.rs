#![deny(missing_docs)]
//! # UBX GPS Parser
//! A limited capability parser for UBX GPS messages.
//!
//! Parses NMEA GGA, GSA, GSV and VTG messages, along with UBX-RXM-RAWX messages.
//! Provides a simple interface to extract timestamp, location, carrier phase
//! and satellite information.
mod nmea;
mod read_until;
mod tec;
mod ubx;
mod uncertain;

use std::io::Read;

pub use nmea::{GnssSatellite, GpsError, NmeaGpsInfo};
pub use ubx::{
    BeidouFreq, CarrierMeas, GalileoFreq, GlonassFreq, GnssFreq, GpsFreq, QzssFreq, SatPathInfo,
    UbxGpsInfo,
};

pub use uncertain::Uncertain;
pub use tec::{TecInfo, TecData};

use nmea::RawNmea;
use ubx::{split_ubx, UbxFormat, UbxRxmRawx};

/// Default delimiter for separating UBX messages in a datafile
pub const DEFAULT_DELIM: [u8; 8] = *b"\r\r\n\n\r\r\n\n";

/// Parse a buffer to extract GPS positional information from NMEA messages only
pub fn parse_nmea(buf: Vec<u8>) -> Result<NmeaGpsInfo, GpsError> {
    let buf = std::str::from_utf8(&buf).map_err(|e| GpsError::ParseError(e.to_string()))?;
    let gpsmsg = RawNmea::parse_str(buf);
    let gpsmsg = NmeaGpsInfo::create(&gpsmsg, false)?;
    Ok(gpsmsg)
}

/// Parse a buffer to extract GPS positional information and satellite carrier phase information.
pub fn parse_messages(buf: Vec<u8>) -> Result<UbxGpsInfo, GpsError> {
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
    let gpsmsg = NmeaGpsInfo::create(&gpsmsg, true)?;
    let gpsinfo = UbxGpsInfo::new(gpsmsg, rxm.pop());
    Ok(gpsinfo)
}

/// Parse a datafile containing multiple UBX messages separated by a pattern
pub fn parse_datafile<T: Read>(
    reader: &mut T,
    pattern: &[u8],
) -> Result<Vec<UbxGpsInfo>, GpsError> {
    let mut reader = read_until::get_reader(reader, pattern);
    let mut buffers = Vec::new();
    loop {
        let mut buf = Vec::with_capacity(2048);
        let n = reader
            .read_to_end(&mut buf)
            .map_err(|e| GpsError::ParseError(e.to_string()))?;
        if n == 0 {
            break;
        }
        buffers.push(parse_messages(buf)?);
    }
    Ok(buffers)
}

mod test {
    #[test]
    fn test_parse() {
        let mut datafile = std::fs::File::open("datafile.bin").unwrap();
        let buffers = super::parse_datafile(&mut datafile, b"\r\r\n\n\r\r\n\n")
            .expect("Failed to parse datafile");
        println!("Length: {}", buffers.len());
    }
}
