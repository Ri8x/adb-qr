use crate::error::AppError;
use png::{BitDepth, ColorType, Encoder};
use qrcodegen::{QrCode, QrCodeEcc};
use rand::{seq::SliceRandom, thread_rng};
use std::env;
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const SERVICE_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const SECRET_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
const TERMINAL_BORDER: i32 = 4;
const TERMINAL_LIGHT: &str = "\x1b[48;2;255;255;255m ";
const TERMINAL_DARK: &str = "\x1b[48;2;0;0;0m ";
const TERMINAL_DARK_TOP: &str = "\x1b[38;2;0;0;0;48;2;255;255;255m▀";
const TERMINAL_DARK_BOTTOM: &str = "\x1b[38;2;255;255;255;48;2;0;0;0m▀";
const TERMINAL_RESET: &str = "\x1b[0m";
const SVG_SCALE: i32 = 8;
const PNG_SCALE: u32 = 8;
const PNG_BORDER: u32 = 4;

#[derive(Debug, Clone)]
pub struct PairingPayload {
    pub service_name: String,
    pub secret: String,
    pub payload: String,
}

impl PairingPayload {
    pub fn generate() -> Self {
        let service_name = env::var("ADB_QR_SERVICE_NAME_OVERRIDE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("studio-{}", random_string(10, SERVICE_ALPHABET)));
        let secret = env::var("ADB_QR_SECRET_OVERRIDE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| random_string(16, SECRET_ALPHABET));
        let payload = build_payload(&service_name, &secret);
        Self {
            service_name,
            secret,
            payload,
        }
    }
}

pub fn build_payload(service_name: &str, secret: &str) -> String {
    format!("WIFI:T:ADB;S:{service_name};P:{secret};;")
}

pub fn render_terminal(payload: &str) -> Result<String, AppError> {
    let qr = encode(payload)?;
    let size = qr.size();
    let mut output = String::new();

    let start = -TERMINAL_BORDER;
    let end = size + TERMINAL_BORDER;

    for top_y in (start..end).step_by(2) {
        for x in -TERMINAL_BORDER..size + TERMINAL_BORDER {
            let top_dark = module_with_border(&qr, x, top_y);
            let bottom_dark = module_with_border(&qr, x, top_y + 1);
            output.push_str(match (top_dark, bottom_dark) {
                (false, false) => TERMINAL_LIGHT,
                (true, true) => TERMINAL_DARK,
                (true, false) => TERMINAL_DARK_TOP,
                (false, true) => TERMINAL_DARK_BOTTOM,
            });
        }
        output.push_str(TERMINAL_RESET);
        output.push('\n');
    }

    Ok(output)
}

pub fn render_svg(payload: &str) -> Result<String, AppError> {
    let qr = encode(payload)?;
    let size = qr.size();
    let border = 4;
    let dimension = (size + border * 2) * SVG_SCALE;
    let mut path = String::new();

    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                let x_pos = (x + border) * SVG_SCALE;
                let y_pos = (y + border) * SVG_SCALE;
                let _ = write!(
                    path,
                    "M{x_pos},{y_pos}h{SVG_SCALE}v{SVG_SCALE}h-{SVG_SCALE}z"
                );
            }
        }
    }

    Ok(format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.1" viewBox="0 0 {dimension} {dimension}">
  <rect width="100%" height="100%" fill="#ffffff"/>
  <path d="{path}" fill="#000000"/>
</svg>
"##
    ))
}

pub fn write_png(payload: &str, path: &Path) -> Result<(), AppError> {
    let qr = encode(payload)?;
    let size = qr.size() as u32;
    let dimension = (size + PNG_BORDER * 2) * PNG_SCALE;
    let mut pixels = vec![255u8; (dimension * dimension) as usize];

    for y in 0..dimension {
        for x in 0..dimension {
            let qr_x = x / PNG_SCALE;
            let qr_y = y / PNG_SCALE;
            let dark = qr_x >= PNG_BORDER
                && qr_y >= PNG_BORDER
                && qr_x < size + PNG_BORDER
                && qr_y < size + PNG_BORDER
                && qr.get_module((qr_x - PNG_BORDER) as i32, (qr_y - PNG_BORDER) as i32);
            pixels[(y * dimension + x) as usize] = if dark { 0 } else { 255 };
        }
    }

    let file = File::create(path)
        .map_err(|err| AppError::internal(format!("failed to create {}: {err}", path.display())))?;
    let writer = BufWriter::new(file);
    let mut encoder = Encoder::new(writer, dimension, dimension);
    encoder.set_color(ColorType::Grayscale);
    encoder.set_depth(BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|err| AppError::internal(format!("failed to write PNG header: {err}")))?;
    writer
        .write_image_data(&pixels)
        .map_err(|err| AppError::internal(format!("failed to write PNG data: {err}")))?;
    Ok(())
}

fn encode(payload: &str) -> Result<QrCode, AppError> {
    QrCode::encode_text(payload, QrCodeEcc::Low)
        .map_err(|err| AppError::internal(format!("failed to encode QR payload: {err:?}")))
}

fn module_with_border(qr: &QrCode, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < qr.size() && y < qr.size() && qr.get_module(x, y)
}

fn random_string(length: usize, alphabet: &[u8]) -> String {
    let mut rng = thread_rng();
    (0..length)
        .map(|_| *alphabet.choose(&mut rng).expect("alphabet must not be empty") as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_payload() {
        assert_eq!(
            build_payload("adb-qr-test1234", "Secret123456789"),
            "WIFI:T:ADB;S:adb-qr-test1234;P:Secret123456789;;"
        );
    }

    #[test]
    fn generated_payload_uses_expected_prefix() {
        let payload = PairingPayload::generate();
        assert!(payload.service_name.starts_with("studio-"));
        assert_eq!(payload.service_name.len(), "studio-".len() + 10);
        assert_eq!(payload.secret.len(), 16);
        assert!(payload.payload.contains(&payload.service_name));
    }

    #[test]
    fn terminal_render_uses_explicit_black_and_white_cells() {
        let rendered = render_terminal("WIFI:T:ADB;S:studio-test1234;P:Secret123456789;;")
            .expect("terminal render");
        assert!(rendered.contains(TERMINAL_DARK));
        assert!(rendered.contains(TERMINAL_LIGHT));
        assert!(rendered.contains(TERMINAL_DARK_TOP));
        assert!(rendered.contains(TERMINAL_DARK_BOTTOM));
        assert!(rendered.contains(TERMINAL_RESET));
    }

    #[test]
    fn terminal_render_packs_two_module_rows_per_line() {
        let payload = "WIFI:T:ADB;S:studio-test1234;P:Secret123456789;;";
        let size = encode(payload).expect("encode").size() + TERMINAL_BORDER * 2;
        let rendered = render_terminal(payload).expect("terminal render");

        assert_eq!(rendered.lines().count() as i32, (size + 1) / 2);
    }

    #[test]
    fn pairing_payload_fits_compact_qr_version() {
        let payload = PairingPayload::generate();
        let qr = encode(&payload.payload).expect("encode");

        assert_eq!(qr.size(), 29);
    }
}
