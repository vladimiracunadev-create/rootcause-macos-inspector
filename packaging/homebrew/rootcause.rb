# Plantilla de cask de Homebrew para RootCause macOS Inspector.
#
# No es un canal oficial de distribución: queda como punto de partida
# documentado. Para usarla necesitas un tap propio y un .dmg publicado con su
# SHA-256 real (el de abajo es un marcador de posición).
#
#   brew tap tu-usuario/tap
#   brew install --cask rootcause
cask "rootcause" do
  version "0.1.0"
  sha256 :no_check

  url "https://github.com/vladimiracunadev-create/rootcause-macos-inspector/releases/download/v#{version}/RootCause-#{version}.dmg"
  name "RootCause macOS Inspector"
  desc "Monitor forense: LaunchAgents/Daemons, procesos, Gatekeeper, XProtect, TCC, red y persistencia"
  homepage "https://github.com/vladimiracunadev-create/rootcause-macos-inspector"

  depends_on macos: ">= :ventura"

  app "RootCause.app"
  binary "#{appdir}/RootCause.app/Contents/MacOS/rootcause", target: "rootcause"

  caveats <<~EOS
    RootCause no está firmado ni notarizado. La primera vez, autorízalo en
    Ajustes del Sistema → Privacidad y seguridad.

    Para auditar los permisos de privacidad (TCC), concédele Acceso total al
    disco en Ajustes del Sistema → Privacidad y seguridad → Acceso total al disco.
  EOS

  zap trash: [
    "~/Library/Application Support/RootCauseInspector",
    "~/Documents/RootCause",
  ]
end
