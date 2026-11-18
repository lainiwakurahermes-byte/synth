cask "synth" do
  version :latest
  sha256 :no_check

  url "https://github.com/lainiwakurahermes-byte/synth/releases/latest/download/Synth.zip"
  name "Synth"
  desc "Learning polysynth"
  homepage "https://github.com/lainiwakurahermes-byte/synth"

  app "Synth.app"

  # The app is not notarized, so Gatekeeper blocks it while the quarantine flag is set.
  postflight do
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/Synth.app"]
  end
end
