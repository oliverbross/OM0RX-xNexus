// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Read-only diagnostic for the draft bridge. No discovery or hardware startup.
use std::{fs::File, io::Read};
use xnexus_xlog_client::BridgeClient;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 || args[0] != "--endpoint" || args[2] != "--token-file" {
        return Err("Usage: xnexus-xlog-client --endpoint http://127.0.0.1:PORT --token-file PATH (draft bridge required)".into());
    }
    let mut token = String::new();
    File::open(&args[3])
        .map_err(|_| "Cannot read pairing-token file")?
        .take(4098)
        .read_to_string(&mut token)
        .map_err(|_| "Invalid pairing-token file")?;
    if token.len() > 4097 {
        return Err("Pairing-token file exceeds the limit".into());
    }
    let mut client = BridgeClient::new(&args[1], token.trim_end_matches(['\r', '\n']))?;
    client.connect()?;
    let snapshot = client.poll()?;
    println!("xLog bridge connected (protocol 1). Revision: {}. Spots: {}. Needed: {}. Radio connected: {}. Rotor connected: {}.", snapshot.revision, snapshot.spots.len(), snapshot.needed().len(), snapshot.station.connected, snapshot.rotor.connected);
    println!("Read-only observation; no station action performed.");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
