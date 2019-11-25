{ pkgs ? import ./pkgs.nix {} }:

with pkgs;

let
  inherit (rust.packages.nightly) rustPlatform;
in

{
  holo-router-agent = buildRustPackage rustPlatform {
    name = "holo-router-agent";
    src = gitignoreSource ./agent;
    cargoSha256 = "0akyl9h6ajih1v4xspwiryzpwm1jwyp4jgqh8z54cb1b9ynn75ia";

    RUST_SODIUM_LIB_DIR = "${libsodium}/lib";
    RUST_SODIUM_SHARED = "1";

    nativeBuildInputs = [ perl ];

    meta.platforms = lib.platforms.linux;
  };

  holo-router-gateway = callPackage ./gateway {};

  holo-router-registry = buildRustPackage rustPlatform {
    name = "holo-router-registry";
    src = gitignoreSource ./registry;
    cargoDir = ".";

    nativeBuildInputs = [
      nodejs
      python
      (wasm-pack.override { inherit rustPlatform; })
    ];
  };
}
