#!/usr/bin/env bash
#
# Generate the Homebrew formula for basalt from the checksums of a release.
#
# Usage: homebrew-formula.sh VERSION CHECKSUM_DIR
#
# VERSION       release version without the leading "v", e.g. 0.12.7
# CHECKSUM_DIR  directory holding the "*.tar.gz.sha256" files of the release
#
# Each checksum file is the output of `shasum -a 256 <archive>`, so the SHA is
# its first whitespace separated field.

set -euo pipefail

version="${1:?version required}"
checksum_dir="${2:?checksum directory required}"

base_url="https://github.com/erikjuhani/basalt/releases/download/basalt/v${version}"

sha256() {
  local target="$1"
  local file="${checksum_dir}/basalt-${version}-${target}.tar.gz.sha256"
  awk '{ print $1 }' "${file}"
}

cat <<FORMULA
class Basalt < Formula
  desc "TUI application for Obsidian notes"
  homepage "https://github.com/erikjuhani/basalt"
  license "GPL-3.0-or-later"

  on_macos do
    on_arm do
      url "${base_url}/basalt-${version}-aarch64-apple-darwin.tar.gz"
      sha256 "$(sha256 aarch64-apple-darwin)"
    end
    on_intel do
      url "${base_url}/basalt-${version}-x86_64-apple-darwin.tar.gz"
      sha256 "$(sha256 x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "${base_url}/basalt-${version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "$(sha256 aarch64-unknown-linux-gnu)"
    end
    on_intel do
      url "${base_url}/basalt-${version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "$(sha256 x86_64-unknown-linux-gnu)"
    end
  end

  def install
    bin.install "basalt"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/basalt --version")
  end
end
FORMULA
