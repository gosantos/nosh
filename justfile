default:
    @just --list

dev:
    cargo watch -x 'run --release --bin nosh'

build:
    cargo build --release

release: build
    @echo "Binary at target/release/nosh"

check:
    cargo clippy -- -D warnings

fmt:
    cargo fmt
