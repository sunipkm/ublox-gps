use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

use ubx::{split_ubx, UbxFormat, UbxRxmRawx};

mod ubx;

fn main() {
    let mut serial_port = serialport::new("/dev/ttyUSB0", 38400)
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
        print!("Read: {} bytes\t", buf.len());
        now = Instant::now();
        println!("Time: {} s", (now - last).as_secs_f32());
        let (ubx, buf) = split_ubx(buf);
        for msg in ubx {
            println!(
                "Class: {:02X} ID: {:02X} Valid: {}",
                msg.class,
                msg.id,
                msg.validate()
            );
            if let Ok(msg) = UbxRxmRawx::from_message(msg) {
                println!("{:?}", msg);
            }
        }
        println!("Remaining: {}", buf.len());
        if let Ok(line) = std::str::from_utf8(&buf) {
            let lines = line.lines();
            for line in lines {
                if let Ok(msg) = parser.parse_sentence(line) {
                    println!("{:?}", msg);
                } else {
                    println!("Failed to parse: {}", line);
                }
            }
        }
        // println!("{:?}", buf);
    }
}

fn parse_messages(buf: &[u8]) {
    // 1. Split the buffer into messages
    // 1a. Find the binary message
}
