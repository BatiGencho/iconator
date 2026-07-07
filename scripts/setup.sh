#!/bin/bash

if ! command -v pre-commit &> /dev/null; then
    brew install pre-commit
fi
pre-commit install

if ! command -v bun &> /dev/null; then
    curl -fsSL https://bun.sh/install | bash
fi

bun install

install_cmd="cargo binstall --force --no-confirm"

cargo install cargo-binstall
$install_cmd cargo-tarpaulin
$install_cmd samply
$install_cmd cargo-watch
$install_cmd knope
$install_cmd cargo-sort
$install_cmd typos-cli
$install_cmd cargo-nextest --secure

cargo install cargo-audit --locked --features=fix --force
cargo install release-plz --locked
cargo install taplo-cli --locked
cargo install bacon --locked
cargo install cargo-machete --locked
# diesel CLI, postgres-only so it doesn't require MySQL/SQLite client libs
cargo install diesel_cli --no-default-features --features postgres --locked
