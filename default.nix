{ pkgs ? import ./nixpkgs.nix {} }:

with pkgs;

let
  inherit (rust.packages.nightly) rustPlatform;
in

{
  holo-router-gateway = buildRustPackage rustPlatform {
    name = "holo-router-gateway";
    src = gitignoreSource ./.;
    cargoDir = "./gateway";

    meta.platforms = lib.platforms.linux;
  };
}
