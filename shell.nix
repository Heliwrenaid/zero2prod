{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustc
    rustfmt
    cargo
    pkg-config
    libiconv
    openssl
    bunyan-rs
  ];

  # Setting the RUST_SRC_PATH environment variable for tools like rust-analyzer
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  shellHook = ''
    export PATH=$PATH:$HOME/.cargo/bin
    source .env
    if ! command -v sqlx > /dev/null; then
      echo "Installing sqlx-cli..."
      cargo install --version='~0.7' sqlx-cli --no-default-features --features rustls,postgres
    fi
  '';
}

# http://localhost:8080/__admin/requests
# http://localhost:8080/__admin/mappings
