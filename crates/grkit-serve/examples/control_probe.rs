//! Exercise the production control transport on a camera or an emulator
//! card: find the mailbox, then read the effect list or set a choice.
use anyhow::{anyhow, ensure, Result};
use grkit_serve::{
    control,
    runtime_backend::{look_key, ControlBackend, RuntimeBackend},
};
use std::{io::Read, path::Path};

const USAGE: &str = "Usage: control_probe [--turn ID on|off | --order ID,ID,...] CARD_ROOT";

fn main() -> Result<()> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let mut turn = None;
    let mut order = None;
    match args.first().map(String::as_str) {
        Some("--turn") => {
            ensure!(args.len() > 3, USAGE);
            turn = Some((args[1].clone(), args[2] == "off"));
            args.drain(..3);
        }
        Some("--order") => {
            ensure!(args.len() > 2, USAGE);
            order = Some(args[1].clone());
            args.drain(..2);
        }
        _ => {}
    }
    ensure!(args.len() == 1, USAGE);
    let mut nonce = [0; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut nonce)?;
    let client = control::discover(Path::new(&args[0]))?;
    let mut backend = ControlBackend::new(client, nonce);
    let problem =
        |problem: grkit_serve::protocol::Problem| anyhow!("{}: {}", problem.code, problem.detail);
    if let Some((id, off)) = turn {
        backend.turn(look_key(&id), off).map_err(problem)?;
    }
    if let Some(ids) = order {
        let keys: Vec<u32> = ids.split(',').map(look_key).collect();
        backend.order(&keys).map_err(problem)?;
    }
    println!("{:#?}", backend.looks().map_err(problem)?);
    Ok(())
}
