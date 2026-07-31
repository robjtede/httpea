import '.toolchain/rust.just'

_list:
    @just --list

toolchain := ""
external-types-toolchain := "nightly-2026-03-20"
# Format project
fmt:
    just --unstable --fmt
    cargo +nightly fmt
    fd --hidden -e=yml --exec-batch prettier --write
    fd --hidden -e=toml --exec-batch taplo format

# Check project
check:
    just --unstable --fmt --check
    cargo +nightly fmt --all --check
    cargo {{ toolchain }} clippy --workspace --all-targets --all-features
    fd --hidden -e=yml --exec-batch prettier --check
    fd --hidden -e=toml --exec-batch taplo format
    fd --hidden -e=toml --exec-batch taplo lint
    cargo shear

[private]
workspace-crate-manifests:
    @cargo metadata --no-deps --format-version=1 \
        | jq -r '(.workspace_root + "/") as $root | .workspace_members as $members | .packages[] | select(.id as $id | $members | index($id)) | .manifest_path | ltrimstr($root)'

# Check crates are not leaking unexpected external types
check-external-types:
    just workspace-crate-manifests \
        | while IFS= read -r manifest; do \
            cargo +{{ external-types-toolchain }} check-external-types --manifest-path "$manifest"; \
        done

# Lint workspace with Clippy
clippy:
    cargo {{ toolchain }} clippy --workspace --all-targets --all-features

# Test workspace without doc tests
[private]
test-no-doc:
    cargo {{ toolchain }} nextest run --workspace --lib --tests --examples

# Test workspace doc tests
[private]
test-doc:
    cargo {{ toolchain }} test --doc --workspace

# Test workspace
[parallel]
test: test-no-doc test-doc

# Test workspace using MSRV
test-msrv:
    @just toolchain={{ msrv_rustup }} test

# Test workspace and generate Codecov coverage file
test-coverage-codecov:
    cargo {{ toolchain }} llvm-cov --workspace --all-features --codecov --output-path codecov.json

# Test workspace and generate LCOV coverage file
test-coverage-lcov:
    cargo {{ toolchain }} llvm-cov --workspace --all-features --lcov --output-path lcov.info

# Document workspace
doc *args:
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --workspace --all-features --no-deps {{ args }}
