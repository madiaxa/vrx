#!/bin/sh
# Runs inside a FreeBSD VM, not on the Linux GitHub host.
set -eu
. "$HOME/.cargo/env"
export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:/usr/local/share/pkgconfig
export CARGO_TARGET_DIR=/tmp/vrx-target
export CARGO_BUILD_JOBS=2
cargo fmt --check
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked --target x86_64-unknown-freebsd
"$CARGO_TARGET_DIR/x86_64-unknown-freebsd/release/vrx" --version
if [ "${VRX_PACKAGE:-false}" = "true" ]; then
    python3 packaging/package_release.py \
        --binary "$CARGO_TARGET_DIR/x86_64-unknown-freebsd/release/vrx" \
        --target x86_64-unknown-freebsd --output dist
fi
