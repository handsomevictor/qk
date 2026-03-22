# RELEASE — Publishing Guide

This document covers how to cut a GitHub Release and publish `qk` to Homebrew.

---

## Overview

```
git tag v0.x.0 && git push origin v0.x.0
        ↓
GitHub Actions builds 5 platform binaries automatically
        ↓
Release page appears at github.com/handsomevictor/qk/releases
        ↓
Copy SHA256 hashes → update homebrew-qk Formula → push
        ↓
Users: brew tap handsomevictor/qk && brew install qk
```

---

## Step 1 — Tag and push

```bash
cargo test                  # make sure all tests pass
git tag v0.1.0
git push origin v0.1.0
```

This triggers `.github/workflows/release.yml`, which builds binaries for all 5 platforms
and uploads them to a new GitHub Release automatically (~10 minutes).

---

## Step 2 — Verify the Release

Go to `https://github.com/handsomevictor/qk/releases`.

You should see a Release named `v0.1.0` with the following assets:

```
qk-v0.1.0-aarch64-apple-darwin.tar.gz
qk-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
qk-v0.1.0-x86_64-apple-darwin.tar.gz
qk-v0.1.0-x86_64-apple-darwin.tar.gz.sha256
qk-v0.1.0-aarch64-unknown-linux-musl.tar.gz
qk-v0.1.0-aarch64-unknown-linux-musl.tar.gz.sha256
qk-v0.1.0-x86_64-unknown-linux-musl.tar.gz
qk-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
qk-v0.1.0-x86_64-pc-windows-msvc.zip
qk-v0.1.0-x86_64-pc-windows-msvc.zip.sha256
```

---

## Step 3 — Set up the Homebrew tap (first time only)

Create a **public** GitHub repository named exactly `homebrew-qk`:

```
https://github.com/new  →  Repository name: homebrew-qk  →  Public
```

Clone it and create the formula directory:

```bash
git clone https://github.com/handsomevictor/homebrew-qk.git
cd homebrew-qk
mkdir Formula
```

---

## Step 4 — Get SHA256 hashes

Fetch the hashes from the release assets:

```bash
VER=v0.1.0
BASE=https://github.com/handsomevictor/qk/releases/download/$VER

curl -sL $BASE/qk-$VER-aarch64-apple-darwin.tar.gz.sha256
curl -sL $BASE/qk-$VER-x86_64-apple-darwin.tar.gz.sha256
curl -sL $BASE/qk-$VER-aarch64-unknown-linux-musl.tar.gz.sha256
curl -sL $BASE/qk-$VER-x86_64-unknown-linux-musl.tar.gz.sha256
```

---

## Step 5 — Write the Formula

Create `Formula/qk.rb` in the `homebrew-qk` repo.
Replace each `SHA256_HERE` with the actual hash from Step 4:

```ruby
class Qk < Formula
  desc "One terminal tool to replace grep, awk, jq, yq, and more"
  homepage "https://github.com/handsomevictor/qk"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/handsomevictor/qk/releases/download/v0.1.0/qk-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_HERE"
    else
      url "https://github.com/handsomevictor/qk/releases/download/v0.1.0/qk-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_HERE"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/handsomevictor/qk/releases/download/v0.1.0/qk-v0.1.0-aarch64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_HERE"
    else
      url "https://github.com/handsomevictor/qk/releases/download/v0.1.0/qk-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "SHA256_HERE"
    end
  end

  def install
    bin.install "qk"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/qk --version")
  end
end
```

Push the formula:

```bash
git add Formula/qk.rb
git commit -m "add qk v0.1.0"
git push
```

Users can now install:

```bash
brew tap handsomevictor/qk
brew install qk
```

---

## Releasing a new version

1. Update `version` in `Cargo.toml`
2. `git tag v0.2.0 && git push origin v0.2.0`
3. Wait for GitHub Actions to finish
4. Fetch new SHA256 hashes (Step 4 above)
5. Update `version`, `url`, and `sha256` in `Formula/qk.rb` and push
