# typed: false
# frozen_string_literal: true

# Homebrew formula for qk — single CLI replacing grep/awk/sed/jq/yq.
#
# To update after a new release:
#   1. Update `url` with the new release archive URL
#   2. Update `sha256` with the checksum from `shasum -a 256 <archive>`
#   3. Update `version`
#
# Installation:
#   brew tap OWNER/qk
#   brew install qk
class Qk < Formula
  desc "Single CLI tool replacing grep / awk / sed / jq / yq / cut / sort"
  homepage "https://github.com/OWNER/qk"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/OWNER/qk/releases/download/v#{version}/qk-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_AARCH64_MACOS_SHA256"
    end
    on_intel do
      url "https://github.com/OWNER/qk/releases/download/v#{version}/qk-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64_MACOS_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/OWNER/qk/releases/download/v#{version}/qk-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_AARCH64_LINUX_SHA256"
    end
    on_intel do
      url "https://github.com/OWNER/qk/releases/download/v#{version}/qk-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_X86_64_LINUX_SHA256"
    end
  end

  def install
    bin.install "qk"
  end

  test do
    # Verify the binary runs and produces valid output
    assert_match "qk", shell_output("#{bin}/qk --help")
    # Verify basic NDJSON parsing works
    output = pipe_output("#{bin}/qk where level=error", '{"level":"error","msg":"ok"}', 0)
    assert_match "error", output
  end
end
