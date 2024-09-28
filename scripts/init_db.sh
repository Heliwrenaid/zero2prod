#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error: sqlx is not installed."
    echo >&2 "Use:"
    echo >&2 "
    cargo install --version='~0.7' sqlx-cli \
    --no-default-features --features rustls,postgres"
    echo >&2 "to install it."
    exit 1
fi

source .env
sqlx database create
sqlx migrate run