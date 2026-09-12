# TODO

## Platform Follow-up

- Validate Linux host discovery and Android SDK path resolution on at least one Debian-based system.
- Validate Windows host discovery and Android SDK path resolution on at least one Windows 11 system.
- Replace the current macOS-first wording in `README.md` once Linux and Windows are manually verified.

## Distribution

- Add GitHub release workflow for macOS signed binaries.
- Add Linux and Windows release artifacts after host validation is complete.
- Publish the checked-in Homebrew formula and verify installation from the public tap on Apple Silicon and Intel.
- Publish the first stable version tag; the tap currently pins the existing `0.1.0-main.2` prerelease.
- Update the formula URL, version, and checksum after each tested release (currently manual).
- Evaluate Scoop and WinGet after Windows validation.

## Testing

- Expand integration coverage beyond Unix shell-based fake `adb`.
- Add Windows-native fake `adb` integration tests.
- Run and record the real-device smoke test documented in `docs/RELEASING.md` on Apple Silicon and Intel.
