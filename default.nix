 { pkgs ? import <nixpkgs> { } }:

with pkgs;
mkShell {
  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    dbus
    gtk4
    gtk-layer-shell
  ];

}
