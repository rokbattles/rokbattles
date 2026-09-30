#!/bin/sh
# Read the resolved crate version so the CLI uses the same binding format.
set -eu

workspace_dir=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
awk '
    /^\[\[package\]\]/ { bindgen = 0 }
    /^name = "wasm-bindgen"$/ { bindgen = 1 }
    bindgen && /^version = / {
        gsub(/"/, "", $3)
        version = $3
        count++
    }
    END {
        if (count != 1) exit 1
        print version
    }
' "$workspace_dir/Cargo.lock" || {
    echo 'Expected one resolved wasm-bindgen version in Cargo.lock' >&2
    exit 1
}
