{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ rustc rustfmt cargo pkg-config libiconv openssl postgresql bunyan-rs dbeaver-bin ];

  # Certain Rust tools won't work without this
  # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  # RUST_TEST_THREADS=1;
}

# docker run -d -p 8080:8080 -v $(pwd)/mappings:/home/wiremock/mappings wiremock/wiremock


# http://localhost:8080/__admin/requests
# http://localhost:8080/__admin/mappings
