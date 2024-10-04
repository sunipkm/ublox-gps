#![deny(missing_docs)]
//! # UBX GPS Parser
//! A limited capability parser for UBX GPS messages.
//!
//! Parses NMEA GGA, GSA, GSV and VTG messages, along with UBX-RXM-RAWX messages.
//! Provides a simple interface to extract timestamp, location, carrier phase
//! and satellite information.
mod nmea;
mod ubx;

pub use nmea::{GnssSatellite, GpsError, NmeaGpsInfo};
pub use ubx::{
    BeidouFreq, CarrierMeas, GalileoFreq, GlonassFreq, GnssFreq, GpsFreq, QzssFreq, UbxGpsInfo,
};

use nmea::RawNmea;
use ubx::{split_ubx, UbxFormat, UbxRxmRawx};

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


