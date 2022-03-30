{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "rit";
  nativeBuildInputs = with pkgs; [
    llvmPackages.clang
    rustup
  ];
}
