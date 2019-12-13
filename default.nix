{ pkgs ? import ./pkgs.nix {} }:

with pkgs;

let
  inherit (rust.packages.nightly) rustPlatform;
in

{
  holo-router-agent = buildRustPackage rustPlatform {
    name = "holo-router-agent";
    src = gitignoreSource ./agent;
    cargoSha256 = "1gy0wad809p5n62kn6knq0xgrfdawcm9cj4z1kv0d7k8sj2cfnv1";

    RUST_SODIUM_LIB_DIR = "${libsodium}/lib";
    RUST_SODIUM_SHARED = "1";

    nativeBuildInputs = [ perl ];

    meta.platforms = lib.platforms.linux;
  };

  holo-router-gateway = callPackage ./gateway {};
}
