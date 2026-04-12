_list:
    @just --list

msrv := `awk '/^\[workspace.package\]/{flag=1; next} /^\[/{flag=0} flag && /^rust-version = / {gsub(/"/, "", $3); print $3; exit}' Cargo.toml`
msrv_rustup := "+" + msrv

# Build workspace
build toolchain="":
    cargo {{ toolchain }} build --workspace --all-targets

# Lint workspace with Clippy
clippy toolchain="":
    cargo {{ toolchain }} clippy --workspace --all-targets

# Test workspace without doc tests
[private]
test-no-doc toolchain="":
    cargo {{ toolchain }} test --workspace --lib --tests --examples
    cargo {{ toolchain }} bench --workspace --no-run

# Test workspace
test toolchain="": (test-no-doc toolchain)
    cargo {{ toolchain }} test --doc --workspace

# Test workspace using MSRV
test-msrv: (test msrv_rustup)

# Document workspace
doc toolchain="":
    RUSTDOCFLAGS="-D warnings" cargo {{ toolchain }} doc --workspace --no-deps

# Check project
check toolchain="":
    just --unstable --fmt --check
    cargo {{ toolchain }} fmt --all --check
    cargo {{ toolchain }} clippy --workspace --all-targets
    cargo {{ toolchain }} check --workspace --all-targets

# Format project
fmt:
    just --unstable --fmt
    cargo +nightly fmt --all
