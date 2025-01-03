class Dofigen < Formula
  desc "Dofigen is a Dockerfile generator using a simplified description in YAML or JSON format"
  homepage "https://github.com/lenra-io/dofigen"
  version "${VERSION}"

  on_macos do
      on_arm do
          @@file_name = "dofigen-macos-aarch64"
          sha256 "${MACOS_ARM_SHA256}"
      end
      on_intel do
          @@file_name = "dofigen-macos-x86_64"
          sha256 "${MACOS_INTEL_SHA256}"
      end
  end
  on_linux do
      on_arm do
          @@file_name = "dofigen-linux-aarch64"
          sha256 "${LINUX_ARM_SHA256}"
      end
      on_intel do
          @@file_name = "dofigen-linux-x86_64"
          sha256 "${LINUX_INTEL_SHA256}"
      end
  end

  url "https://github.com/lenra-io/dofigen/releases/download/v#{version}/#{@@file_name}"

  def install
    bin.install "#{@@file_name}" => "dofigen"
  end
  test do
    system "#{bin}/dofigen  --version"
    expected_version = "dofigen #{self.version}"
    actual_version = shell_output("#{bin}/dofigen --version").strip
    assert_match expected_version, actual_version
  end
end