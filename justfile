default:
    @just --list

dev:
    cargo watch -x 'run --release'

build:
    cargo build --release

release: build
    @echo "Binary at target/release/tui-todo"

check:
    cargo clippy -- -D warnings

.fmt:
    cargo fmt
