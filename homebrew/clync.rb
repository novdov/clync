class Clync < Formula
  desc "Claude Code 설정 동기화 도구"
  homepage "https://github.com/novdov/clync"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/novdov/clync/releases/download/v#{version}/clync-darwin-arm64"
      sha256 "PLACEHOLDER_SHA256_ARM64"

      def install
        bin.install "clync-darwin-arm64" => "clync"
      end
    end

    on_intel do
      url "https://github.com/novdov/clync/releases/download/v#{version}/clync-darwin-x64"
      sha256 "PLACEHOLDER_SHA256_X64"

      def install
        bin.install "clync-darwin-x64" => "clync"
      end
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/novdov/clync/releases/download/v#{version}/clync-linux-x64"
      sha256 "PLACEHOLDER_SHA256_LINUX"

      def install
        bin.install "clync-linux-x64" => "clync"
      end
    end
  end

  test do
    assert_match "clync", shell_output("#{bin}/clync --version")
  end
end
