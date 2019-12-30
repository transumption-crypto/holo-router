{ pkgs ? import ./nixpkgs.nix {} }:

with pkgs;

let
  inherit (rust.packages.nightly) rustPlatform;
in

{
  holo-router-agent = buildRustPackage rustPlatform {
    name = "holo-router-agent";
    src = gitignoreSource ./agent;
    cargoDir = ".";

    nativeBuildInputs = [ pkgconfig ];
    buildInputs = [ openssl ];

    meta.platforms = lib.platforms.linux;
  };

  holo-router-gateway = callPackage ./gateway {};
}
