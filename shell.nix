{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "rit";
  nativeBuildInputs = with pkgs; [
    llvmPackages.clang
    rustup
    cargo-nextest
    cargo-edit
    git
  ];
  RIT_AUTHOR_NAME="Jamie Quigley";
  RIT_AUTHOR_EMAIL="jamie@quigley.xyz";
}
