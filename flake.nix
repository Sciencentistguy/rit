{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        config.allowUnfree = true;
        inherit system;
      };
      inherit (pkgs) lib;
      fenixStable = fenix.packages.${system}.stable;
      rustToolchain = fenixStable.toolchain;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      };

      rit = {
        rustPlatform,
        lib,
      }:
        rustPlatform.buildRustPackage {
          name = "rit";
          src = lib.cleanSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          buildInputs = with pkgs; (
            [
            ]
            ++ lib.optionals (stdenv.isDarwin) [
              libiconv
            ]
          );

          meta = with lib; {
            license = licenses.mpl20;
            homepage = "https://github.com/Sciencentistguy/rit";
          };
        };
    in rec {
      packages.rit = pkgs.callPackage rit {
        inherit rustPlatform;
      };

      packages.default = self.packages.${system}.rit;

      devShells.default = pkgs.mkShell {
        inputsFrom = [
          packages.rit
        ];
        RUST_SRC_PATH = "${fenixStable.rust-src}/lib/rustlib/src/rust/library";
      };
    });
}
