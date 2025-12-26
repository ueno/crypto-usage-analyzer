#!/bin/sh

export MESON_BUILD_ROOT="$1"
export MESON_SOURCE_ROOT="$2"
export OUTPUT="$3"
export BUILDTYPE="$4"
export APP_ID="$5"

if [ "$BUILDTYPE" = "release" ]
then
    echo "Building in release mode"
    cargo build --manifest-path "$MESON_SOURCE_ROOT"/Cargo.toml --release --target-dir "$MESON_BUILD_ROOT"/target
    cp "$MESON_BUILD_ROOT/target/release/crypto-usage-analyzer" "$OUTPUT"
else
    echo "Building in debug mode"
    cargo build --manifest-path "$MESON_SOURCE_ROOT"/Cargo.toml --target-dir "$MESON_BUILD_ROOT"/target
    cp "$MESON_BUILD_ROOT/target/debug/crypto-usage-analyzer" "$OUTPUT"
fi
