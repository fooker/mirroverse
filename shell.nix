{ }:

let
  mozillaOverlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  pkgs = import <nixpkgs> {
    overlays = [ mozillaOverlay ];
  };
  rustChannel = pkgs.rustChannelOf { date = "2021-11-05"; channel = "nightly"; };

  lib = pkgs.lib;

in pkgs.mkShell {
  buildInputs = with pkgs; [
    rustChannel.rust
    rustChannel.rust-src
    rustChannel.cargo
    pkg-config
  ];

  RUST_BACKTRACE = 1;
  RUST_SRC = "${rustChannel.rust-src}/lib/rustlib/src/rust";
}
