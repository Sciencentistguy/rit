{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    ...
  }:
    {
      overlay = final: prev: {
        rit = self.packages.${prev.system}.default;
      };
    }
    // flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = crane.lib.${system};

        rit = craneLib.buildPackage {
          name = "rit";
          src = craneLib.cleanCargoSource ./.;
          buildInputs = with pkgs; [git]; # for tests
        };
      in {
        packages.rit = rit;

        packages.default = self.packages.${system}.rit;
        devShells.default = self.packages.${system}.default.overrideAttrs (super: {
          nativeBuildInputs = with pkgs;
            super.nativeBuildInputs
            ++ [
              cargo-edit
              cargo-flamegraph
              clippy
              rustc
              rustfmt
            ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        });
      }
    );
}
