_list:
    @just --list

msrv := `awk '/^\[workspace.package\]/{flag=1; next} /^\[/{flag=0} flag && /^rust-version = / {gsub(/"/, "", $3); print $3; exit}' Cargo.toml`
msrv_rustup := "+" + msrv

# Format project
fmt:
    just --unstable --fmt
    cargo +nightly fmt

# Check project
check toolchain="":
    just --unstable --fmt --check
    cargo {{ toolchain }} fmt --all --check
    cargo {{ toolchain }} clippy --workspace --all-targets
    cargo {{ toolchain }} check --workspace --all-targets

# Lint workspace with Clippy
clippy toolchain="":
    cargo {{ toolchain }} clippy --workspace --all-targets --all-features

# Test workspace without doc tests
[private]
test-no-doc toolchain="":
    cargo {{ toolchain }} nextest run --workspace --lib --tests --examples

# Test workspace
test toolchain="": (test-no-doc toolchain)
    cargo {{ toolchain }} test --doc --workspace

# Test workspace using MSRV
test-msrv: (test msrv_rustup)

# Document workspace
doc *args:
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --workspace --no-deps {{ args }}
