use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

use nmea_parser::{NmeaParser, ParsedMessage};
use ubx::{split_ubx, UbxFormat, UbxRxmRawx};

pub mod ubx;
pub mod nmea;

fn main() {
    let mut serial_port = serialport::new("/dev/ttyUSB0", 115200)
        .open()
        .expect("Failed to open serial port");
    serial_port
        .set_timeout(Duration::from_secs_f32(0.1))
        .expect("Failed to set timeout");
    let mut parser = nmea_parser::NmeaParser::new();
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
        println!("----------------------------------------");
        print!("Read: {} bytes\t", buf.len());
        now = Instant::now();
        println!("Time: {} s", (now - last).as_secs_f32());
        let (rxm, nmea) = parse_messages(buf, &mut parser);
        for msg in nmea {
            // println!("{:#?}", msg);
        }
        for mut msg in rxm {
            msg.remove_single_band();
            // println!("{:#?}", msg);
        }
        println!("++++++++++++++++++++++++++++++++++++++++");
        // println!("{:?}", buf);
    }
}

fn parse_messages(buf: Vec<u8>, parser: &mut NmeaParser) -> (Vec<UbxRxmRawx>, Vec<ParsedMessage>) {
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
    let mut nmea = Vec::new();
    if let Ok(line) = std::str::from_utf8(&buf) {
        println!("{}", line);
        let lines = line.lines();
        for line in lines {
            if let Ok(msg) = parser.parse_sentence(line) {
                match &msg {
                    ParsedMessage::Incomplete => {
                        println!("{line}:\tIncomplete");
                        // continue;
                    }
                    _ => {

                    }
                };
                nmea.push(msg);
            } else {
                println!("\tFailed to parse");
            }
        }
    }
    (rxm, nmea)
}
