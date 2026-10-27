# editlicious – Homebrew cask (distribution/README.md).
#
# NOT for homebrew/homebrew-cask: since 1 September 2026 the official tap no longer supports casks that fail
# Gatekeeper (code signing + notarization); v1.0.0 is unsigned. Use it in an own tap
# (github.com/krocon/homebrew-tap → Casks/editlicious.rb) only after Marc agrees – users then have to allow the
# unsigned app themselves (right-click → Open). Submit to the official tap once a signed + notarized release exists,
# after `brew audit --new --cask editlicious` and `brew style`.
cask "editlicious" do
  version "1.0.0"
  sha256 "4ff4af99be572c636b260e932d7651296a7f8acc4703382408978b20a5d2320f"

  url "https://github.com/krocon/editlicious/releases/download/v#{version}/editlicious_#{version}_universal.dmg",
      verified: "github.com/krocon/editlicious/"
  name "editlicious"
  desc "Calm, keyboard-first text editor for huge files"
  homepage "https://krocon.de/products/editlicious/"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: ">= :monterey"

  app "editlicious.app"
  # MCP sidecar (AI control, off by default, docs/mcp.md) – on PATH for `claude mcp add editlicious -- editlicious-mcp`
  binary "#{appdir}/editlicious.app/Contents/MacOS/editlicious-mcp"

  # Tauri stores config/cache under the bundle identifier – verify the paths on a Mac before submitting.
  zap trash: [
    "~/Library/Application Support/io.github.krocon.editlicious",
    "~/Library/Caches/io.github.krocon.editlicious",
    "~/Library/Saved Application State/io.github.krocon.editlicious.savedState",
    "~/Library/WebKit/io.github.krocon.editlicious",
  ]
end
