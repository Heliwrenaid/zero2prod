let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rustVersion = "1.78.0";
  rust = pkgs.rust-bin.stable.${rustVersion}.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
    ];
  };
in
pkgs.mkShell {
  buildInputs = [
    rust
  ] ++ (with pkgs; [
    # gcc
    # rustfmt
    # clippy
    pkg-config
    libiconv
    openssl
    postgresql
    dbeaver
  ]);
  RUST_BACKTRACE = 1;
}