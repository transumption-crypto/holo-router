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
    date = "2019-08-21";
    sha256 = "0idc58ikv5lz7f8pvpv0zxrfcpbs1im24h5jh1crh5yfxc5rimg5";
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
    cargoDir = ".";
  };

  holo-router-proxy = buildRustPackage rustPlatform {
    name = "holo-router-proxy";
    src = gitignoreSource ./proxy;
    cargoDir = ".";
  };

  holo-router-registry = buildRustPackage rustPlatform {
    name = "holo-router-registry";
    src = gitignoreSource ./registry;
    cargoDir = ".";
  };
}
