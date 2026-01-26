class Claudy < Formula
  desc "Claude Code 설정 동기화 도구"
  homepage "https://github.com/novdov/claudy"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/novdov/claudy/releases/download/v#{version}/claudy-darwin-arm64"
      sha256 "PLACEHOLDER_SHA256_ARM64"

      def install
        bin.install "claudy-darwin-arm64" => "claudy"
      end
    end

    on_intel do
      url "https://github.com/novdov/claudy/releases/download/v#{version}/claudy-darwin-x64"
      sha256 "PLACEHOLDER_SHA256_X64"

      def install
        bin.install "claudy-darwin-x64" => "claudy"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/novdov/claudy/releases/download/v#{version}/claudy-linux-x64"
      sha256 "PLACEHOLDER_SHA256_LINUX"

      def install
        bin.install "claudy-linux-x64" => "claudy"
      end
    end
  end

  test do
    assert_match "claudy", shell_output("#{bin}/claudy --version")
  end
end
