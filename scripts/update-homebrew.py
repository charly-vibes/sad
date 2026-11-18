"""Purpose: create or update the Homebrew formula for sad from release checksums.

Responsibilities: parse checksums.txt, render Formula/sad.rb with the four
platform URLs and shas, and report each platform digest.

Rationale: the release workflow bumps the tap from the published artifacts, so
the formula is generated from the same bytes users download instead of being
hand-edited.
"""
import sys
import os

formula_path = sys.argv[1]
version = sys.argv[2]
tag = sys.argv[3]
checksums_path = sys.argv[4]

shas = {}
with open(checksums_path) as f:
    for line in f:
        parts = line.split()
        if len(parts) == 2:
            sha, name = parts
            for platform in ["darwin_arm64", "darwin_amd64", "linux_arm64", "linux_amd64"]:
                if platform in name:
                    shas[platform] = sha

base = f"https://github.com/charly-vibes/sad/releases/download/{tag}"

formula = f"""\
# typed: false
# frozen_string_literal: true

class Sad < Formula
  desc "Documentation gate — code should speak for itself; prose means refactor"
  homepage "https://github.com/charly-vibes/sad"
  version "{version}"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "{base}/sad_{version}_darwin_arm64.tar.gz"
      sha256 "{shas['darwin_arm64']}"
    end
    on_intel do
      url "{base}/sad_{version}_darwin_amd64.tar.gz"
      sha256 "{shas['darwin_amd64']}"
    end
  end

  on_linux do
    on_arm do
      if Hardware::CPU.is_64_bit?
        url "{base}/sad_{version}_linux_arm64.tar.gz"
        sha256 "{shas['linux_arm64']}"
      end
    end
    on_intel do
      url "{base}/sad_{version}_linux_amd64.tar.gz"
      sha256 "{shas['linux_amd64']}"
    end
  end

  def install
    bin.install "sad"
  end

  test do
    system "#{{bin}}/sad", "--version"
  end
end
"""

os.makedirs(os.path.dirname(formula_path), exist_ok=True)
with open(formula_path, "w") as f:
    f.write(formula)

print(f"Wrote {formula_path} (version {version})")
for p, s in shas.items():
    print(f"  {p}: {s[:16]}...")