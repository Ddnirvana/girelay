default:
    just validate

fmt:
    cargo fmt --check

clippy:
    cargo clippy --all-targets -- -D warnings

test:
    cargo test

validate:
    bash scripts/validate.sh

demo:
    cargo build
    PATH="{{justfile_directory()}}/target/debug:$PATH" bash scripts/demo.sh

release-check:
    bash scripts/release-check.sh
