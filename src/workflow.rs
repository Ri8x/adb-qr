use crate::adb::Adb;
use crate::cli::{Cli, Commands, PairArgs, QrArgs};
use crate::error::AppError;
use crate::qr;
use std::collections::HashSet;
use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

pub fn execute(cli: Cli) -> i32 {
    let result = match cli.command {
        Some(Commands::Pair(args)) => handle_pair(args),
        Some(Commands::Qr(args)) => handle_qr(args),
        None => handle_pair(cli.pair),
    };

    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {}", error.message);
            error.exit_code()
        }
    }
}

fn handle_pair(args: PairArgs) -> Result<i32, AppError> {
    let adb = Adb::resolve(args.adb_path.clone())?;
    adb.version_text()?;
    adb.mdns_check()?;
    let baseline = adb.device_set().unwrap_or_default();
    let connected = run_qr_pair(&adb, &args, &baseline)?;

    println!();
    if connected {
        println!("Pairing succeeded and the device is visible in adb.");
    } else {
        println!("Pairing succeeded, but no wireless connection service was discovered.");
        println!("Keep Wireless Debugging enabled, then run `adb connect <device-ip>:<connection-port>`.");
        println!("Use the IP address and port shown on the main Wireless Debugging screen, not the pairing port.");
    }

    Ok(0)
}

fn handle_qr(args: QrArgs) -> Result<i32, AppError> {
    let payload = qr::PairingPayload::generate();

    if let Some(path) = &args.svg {
        let svg = qr::render_svg(&payload.payload)?;
        std::fs::write(path, svg).map_err(|err| {
            AppError::internal(format!("failed to write SVG to {}: {err}", path.display()))
        })?;
    }

    if let Some(path) = &args.png {
        qr::write_png(&payload.payload, path)?;
    }

    println!("Service name: {}", payload.service_name);
    println!("Secret: {}", payload.secret);
    if std::io::stdout().is_terminal() {
        println!("{}", qr::render_terminal(&payload.payload)?);
    } else {
        println!("Terminal QR output is not supported on this stdout target.");
    }

    if args.print_payload || !std::io::stdout().is_terminal() {
        println!("Payload: {}", payload.payload);
    }

    if let Some(path) = &args.svg {
        println!("Wrote SVG: {}", path.display());
    }
    if let Some(path) = &args.png {
        println!("Wrote PNG: {}", path.display());
    }

    Ok(0)
}

fn run_qr_pair(adb: &Adb, args: &PairArgs, baseline: &HashSet<String>) -> Result<bool, AppError> {
    let payload = qr::PairingPayload::generate();
    let png_path = write_pair_png(&payload.payload)?;
    let pairing_baseline = adb.pairing_service_names().unwrap_or_default();

    println!("Open Wireless Debugging on your device and choose \"Pair device with QR code\".");
    println!("QR image: {}", png_path.display());
    println!("Scan this QR code:");
    if std::io::stdout().is_terminal() {
        println!("{}", qr::render_terminal(&payload.payload)?);
    } else {
        println!("Payload: {}", payload.payload);
    }
    println!(
        "Waiting for the pairing service `{}` to appear via adb mDNS...",
        payload.service_name
    );

    let service = adb.wait_for_pairing_service(
        &payload.service_name,
        &pairing_baseline,
        Duration::from_secs(args.timeout),
    )?;
    if service.name != payload.service_name {
        println!(
            "Note: adb advertised the pairing service as `{}` rather than `{}`; trying it.",
            service.name, payload.service_name
        );
    }
    println!("Pairing with {}...", service.endpoint);
    adb.pair(&service.endpoint, &payload.secret)?;
    if adb.wait_for_device(baseline, Duration::from_secs(2))? {
        return Ok(true);
    }

    println!("Pairing succeeded. Looking for the wireless connection service...");
    let connect_service =
        match adb.wait_for_connect_service(&service.endpoint, Duration::from_secs(8)) {
            Ok(service) => service,
            Err(error) => {
                eprintln!("Warning: {}", error.message);
                return Ok(false);
            }
        };

    let Some(connect_service) = connect_service else {
        return Ok(false);
    };

    println!("Connecting to {}...", connect_service.endpoint);
    if let Err(error) = adb.connect(&connect_service.endpoint) {
        eprintln!("Warning: {}", error.message);
        return Ok(false);
    }

    adb.wait_for_device(baseline, Duration::from_secs(5))
}

fn write_pair_png(payload: &str) -> Result<PathBuf, AppError> {
    let path = env::temp_dir().join("adb-qr-pairing.png");
    qr::write_png(payload, &path)?;
    Ok(path)
}
