#!/usr/bin/env -S just --justfile

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set shell := ["bash", "-cu"]

_default:
  @just --list -u

init:
  cargo bininstall cargo-shear@1.11.1 -y

check:
  just fmt
  just test
  just lint

fmt:
  cargo shear --fix
  cargo fmt --all
  pnpm run format

test:
  cargo test

lint:
  cargo clippy --workspace --all-targets --all-features -- --deny warnings
