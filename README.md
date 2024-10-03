## About
This repository contains the code from the book [Zero To Production In Rust](https://www.zero2prod.com/index.html): an opinionated introduction to backend development using Rust.
Compared to the code in the book, I made some improvements and implemented additional functionalities:
*  local development improvements, including:
    -  using docker-compose instead of shell scripts
    -  mocking email service with [WireMock](https://github.com/wiremock/wiremock)
    -  NixOS dev shell
    -  simple shell scripts for calling API
*  invite new collabolators
   - app can have one admin and mutliple collabolators
   - only admin can invite new collabolators, by sending to them account activation emails
   - collabolator durning account activation can provide username and password
* retry (with exponential backoff) failed emails deliveries on transient errors
* automatically clean expired idempotency keys from db
* pin Rust toolchain version in main pipeline, making it more deterministic (especially `clippy`)

## Development
### NixOS
```bash
nix-shell
docker compose up -d
./scripts/init_db.sh
cargo run | bunyan
```

### Other systems
Install [Rust](https://www.rust-lang.org/) and few system packages needed for compile the project (see [shell.nix](shell.nix) `nativeBuildInputs` section).
```bash
source .env
cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,postgres
docker compose up -d
./scripts/init_db.sh
cargo run | bunyan
```
