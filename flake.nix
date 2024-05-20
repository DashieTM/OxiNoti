{
  description = "A work in progress notification daemon made with rust and gtk.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";
    };
  };

  outputs = inputs @ { self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem =
        { config
        , self'
        , inputs'
        , pkgs
        , system
        , ...
        }:
        {
          _module.args.pkgs = import self.inputs.nixpkgs {
            inherit system;
            overlays = [
              (import
                inputs.rust-overlay
              )
            ];
          };
          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self'.packages;
            packages = with pkgs; [
              (rust-bin.selectLatestNightlyWith
                (toolchain: toolchain.default))
            ];
          };

          packages =
            let
              lockFile = ./Cargo.lock;
            in
            rec {
              oxinoti = pkgs.callPackage ./nix/default.nix { inherit inputs lockFile; };
              default = oxinoti;
            };
        };
      flake = _: rec {
        nixosModules.home-manager = homeManagerModules.default;
        homeManagerModules = rec {
          oxinoti = import ./nix/hm.nix inputs.self;
          default = oxinoti;
        };
      };
    };
}
