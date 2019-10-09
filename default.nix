{ pkgs ? import ./pkgs.nix {} }:

with pkgs;

let
  # https://github.com/mozilla/nixpkgs-mozilla/pull/200
  nixpkgs-mozilla = fetchTarball {
    url = "https://github.com/mozilla/nixpkgs-mozilla/archive/24d112e4895f081700ab910889818c5e189f4d69.tar.gz";
    sha256 = "0kvwbnwxbqhc3c3hn121c897m89d9wy02s8xcnrvqk9c96fj83qw";
  };

  inherit (callPackage "${nixpkgs-mozilla}/package-set.nix" {}) rustChannelOf;

  rustChannel = rustChannelOf {
    channel = "nightly";
    date = "2019-09-16";
    sha256 = "1k75ps2ymjr1mz1p751ifmsxwcjyr4k0i87jsmiaj7rblhgfvcan";
  };

  rustPlatform = makeRustPlatform {
    cargo = rustChannel.rust;
    rustc = rustChannel.rust;
  };
in

{
  holo-router-agent = buildRustPackage rustPlatform {
    name = "holo-router-agent";
    src = gitignoreSource ./agent;
    cargoSha256 = "0akyl9h6ajih1v4xspwiryzpwm1jwyp4jgqh8z54cb1b9ynn75ia";

    nativeBuildInputs = [ pkgconfig ];
    buildInputs = [ openssl ];
  };

  holo-router-gateway = buildRustPackage rustPlatform {
    name = "holo-router-gateway";
    src = gitignoreSource ./gateway;
    cargoDir = ".";
  };

  holo-router-registry = buildRustPackage rustPlatform {
    name = "holo-router-registry";
    src = gitignoreSource ./registry;
    cargoDir = ".";
  };
}
