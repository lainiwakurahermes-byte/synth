class Synth < Formula
  desc "Learning polysynth"
  homepage "https://github.com/lainiwakurahermes-byte/synth"
  head "https://github.com/lainiwakurahermes-byte/synth.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end
end
