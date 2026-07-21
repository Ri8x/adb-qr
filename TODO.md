# TODO

## Platform Follow-up

- Validate Linux host discovery and Android SDK path resolution on at least one Debian-based system.
- Validate Windows host discovery and Android SDK path resolution on at least one Windows 11 system.
- Replace the current macOS-first wording in `README.md` once Linux and Windows are manually verified.

## Distribution

- Add GitHub release workflow for macOS signed binaries.
- Add Linux and Windows release artifacts after host validation is complete.
- Add Homebrew tap formula once the binary release flow is stable.
- Evaluate Scoop and WinGet after Windows validation.

## Testing

- Expand integration coverage beyond Unix shell-based fake `adb`.
- Add Windows-native fake `adb` integration tests.
- Add real-device smoke-test documentation for contributors.
