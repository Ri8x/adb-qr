# adb-qr

`adb-qr` is a small CLI for Android 11+ Wireless Debugging. It generates the ADB pairing QR payload, renders it in the terminal, and runs the basic QR pairing flow with `adb`.

## What It Does

- `adb-qr` shows a QR code and waits for the device pairing service
- `adb-qr` also writes a PNG copy of the QR to the system temp directory for easier scanning
- `adb-qr qr` shows the pairing QR without running `adb pair`
- optional SVG and PNG export for the QR code

## Support Policy

- Official target: Android 11+ Wireless Debugging
- Primary flow: QR-based pairing
- Older Android `adb tcpip` flows are out of scope

## Installation on macOS (Homebrew)

Install on Apple Silicon or Intel Macs with macOS 11 or later:

```bash
brew tap ri8x/adb-qr https://github.com/Ri8x/adb-qr
brew install --cask android-platform-tools
brew install ri8x/adb-qr/adb-qr
adb-qr
```

Skip the platform-tools install if you already have `adb` available. Homebrew
links `adb-qr` into its bin directory, so you can run it from any directory with
no Rust toolchain or manual `sudo` copy. Homebrew must already be installed and
on your `PATH`; follow [Homebrew's setup instructions](https://brew.sh).

The project-maintained tap verifies downloads with SHA-256 checksums. The
executable is not Developer ID signed or notarized.

To update or remove it:

```bash
brew update
brew upgrade ri8x/adb-qr/adb-qr
brew uninstall ri8x/adb-qr/adb-qr
```

### First pairing

1. Connect your Mac and Android 11+ phone to the same Wi-Fi network.
2. Enable Developer options on the phone, then enable Wireless debugging.
3. Run `adb-qr` on your Mac.
4. On the phone, select **Pair device with QR code** and scan the terminal QR.
5. Confirm the connection with `adb devices`.

If the terminal QR is hard to scan, open the PNG path printed by the command.

### Manual download

Download the latest macOS archive and its `.sha256` file from
[GitHub Releases](https://github.com/Ri8x/adb-qr/releases). Each archive contains
the universal `adb-qr` executable for Apple Silicon and Intel Macs, this README,
and the license.

Verify and extract the download, replacing `<release>` with the downloaded
release name:

```bash
shasum -a 256 -c adb-qr-<release>-macos-universal.tar.gz.sha256
tar -xzf adb-qr-<release>-macos-universal.tar.gz
sudo install -m 0755 adb-qr /usr/local/bin/adb-qr
adb-qr --version
```

`adb-qr` is currently unsigned, so macOS may require confirmation before it can
run. The tool also requires `adb` from Android SDK Platform Tools to be installed.

## Development

Build and test locally with the standard Rust toolchain.

```bash
cargo test
cargo run -- --help
cargo build --release
```

### Runtime dependency

`adb-qr` does not bundle platform-tools. You still need `adb` installed and available through:

- `PATH`
- `--adb-path`
- common Android SDK install locations

## Commands

### `adb-qr`

Guided QR pairing flow.

```bash
adb-qr
adb-qr --timeout 120
adb-qr --adb-path /path/to/adb
```

When you run `adb-qr`, it also writes a PNG copy of the pairing QR to your temp directory as `adb-qr-pairing.png`. If scanning the terminal-rendered QR is unreliable, scan that PNG instead.

### `adb-qr qr`

Generate a QR payload without attempting pairing.

```bash
adb-qr qr
adb-qr qr --svg pairing.svg --png pairing.png --print-payload
```

## Support Matrix

| Host OS | Status |
| --- | --- |
| macOS | First release target |
| Linux | Planned |
| Windows | Planned |

| Device | Status |
| --- | --- |
| Android 11+ with Wireless Debugging | Supported target |
| Android 10 and below | Not automated by this tool |

## Troubleshooting

- If `adb-qr` cannot find `adb`, pass `--adb-path` or add your Android SDK platform-tools directory to `PATH`.
- If QR pairing times out, keep the phone on the Wireless Debugging pairing screen and confirm host and device are on the same network.
- If pairing succeeds but the device does not appear immediately, run `adb devices` after a few seconds. The tool reports that state as a partial success instead of silently failing.
- Some OEM skins expose Wireless Debugging behind additional developer-option menus. The tool cannot enable that setting for you.

## Contributing

See [TODO.md](TODO.md) for planned work and the [release guide](docs/RELEASING.md)
for packaging and release validation.
