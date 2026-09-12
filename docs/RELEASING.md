# Release guide

Pushes to `main` produce development prereleases. For a stable release, set the
version in `Cargo.toml` (and update `Cargo.lock`), then push the matching tag,
for example `v0.1.0`. The release workflow rejects tags that do not match the
package version and builds the universal macOS archive plus its checksum.

After verifying that release, update `url`, `version`, and `sha256` in
`Formula/adb-qr.rb` to the published archive and commit the formula change.
Do not point the formula at an unpublished release or a moving latest URL.
CI installs the formula and exercises QR generation through `brew test`.
Users receive the updated formula with `brew update` and `brew upgrade`.

Before declaring a release device-tested, run the [first-pairing steps](../README.md#first-pairing) on
an Android 11+ device, confirm `adb devices` reports it as `device`, and test a
second run with an already paired phone. Record macOS version, CPU architecture,
Android version, and platform-tools version in the release notes. Automated
tests use a fake ADB and do not prove real-device or network compatibility.
