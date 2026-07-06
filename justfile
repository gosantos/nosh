default:
    @just --list

dev:
    cargo watch -x 'run --release --bin nosh'

build:
    cargo build --release

check:
    cargo clippy -- -D warnings

fmt:
    cargo fmt

bump version:
    #!/usr/bin/env bash
    set -euo pipefail
    version="$1"
    current=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

    case "$version" in
        patch|minor|major)
            IFS='.' read -r major minor patch <<< "$current"
            case "$version" in
                major) major=$((major + 1)); minor=0; patch=0 ;;
                minor) minor=$((minor + 1)); patch=0 ;;
                patch) patch=$((patch + 1)) ;;
            esac
            new="${major}.${minor}.${patch}"
            ;;
        *)
            new="$version"
            ;;
    esac

    if [ "$new" = "$current" ]; then
        echo "already at version $current"
        exit 0
    fi

    sed -i '' "s/^version = \"$current\"/version = \"$new\"/" Cargo.toml
    cargo check --quiet
    git add Cargo.toml Cargo.lock
    git commit -m "chore: bump version to $new"
    git tag "v$new"
    git push origin master
    git push origin "v$new"
    echo "released v$new"
