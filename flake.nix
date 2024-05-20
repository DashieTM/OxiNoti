{
  description = "A work in progress notification daemon made with rust and gtk.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { flake-parts, ... }:
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
          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self'.packages;
            packages = with pkgs; [
              cargo
              rustc
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
