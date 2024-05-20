{ rustPlatform
, rust-bin
, pkg-config
, wrapGAppsHook4
, gtk3
, gtk-layer-shell
, dbus
, lib
, lockFile
, ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in
rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = ../.;

  buildInputs = [
    pkg-config
    gtk3
    gtk-layer-shell
    dbus
  ];

  cargoLock = {
    inherit lockFile;
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
    # (rust-bin.selectLatestNightlyWith
      # (toolchain: toolchain.default))
    rust-bin.nightly."2024-05-10".default
  ];

  copyLibs = true;

  meta = with lib; {
    description = "A small, simple calculator written in rust/gtk4";
    homepage = "https://github.com/DashieTM/OxiNoti";
    changelog = "https://github.com/DashieTM/OxiNoti/releases/tag/${version}";
    license = licenses.gpl3;
    maintainers = with maintainers; [ DashieTM ];
    mainProgram = "oxinoti";
  };
}
