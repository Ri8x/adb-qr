class AdbQr < Formula
  desc "Pair Android devices over wireless ADB using a QR code"
  homepage "https://github.com/Ri8x/adb-qr"
  url "https://github.com/Ri8x/adb-qr/releases/download/v0.1.1/adb-qr-v0.1.1-macos-universal.tar.gz"
  version "0.1.1"
  sha256 "0a11f943a1cdf4bbfd7eaa72115e9767b67079859b757996fd454290232ef571"
  license "MIT"

  depends_on macos: :big_sur

  def install
    bin.install "adb-qr"
  end

  def caveats
    <<~EOS
      Install ADB if it is not already available:
        brew install --cask android-platform-tools

      Run adb-qr, then on your Android 11+ phone open Developer options >
      Wireless debugging > Pair device with QR code. Use the same Wi-Fi network.
    EOS
  end

  test do
    assert_match version.to_s.split("-").first, shell_output("#{bin}/adb-qr --version")
    output = shell_output("#{bin}/adb-qr qr --png pairing.png --print-payload")
    assert_match "WIFI:T:ADB;", output
    assert_equal "\x89PNG\r\n\x1a\n".b, (testpath/"pairing.png").binread(8)
  end
end
