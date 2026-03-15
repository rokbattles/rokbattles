#!/usr/bin/env -S just --justfile

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set shell := ["bash", "-cu"]

_default:
  @just --list -u

init:
  cargo bininstall cargo-shear@1.11.1 -y

ready:
  git diff --exit-code --quiet
  just fmt
  just check
  just test
  just lint

fmt:
  cargo shear --fix
  cargo fmt --all
  pnpm run format

check:
  cargo check --workspace --all-features --all-targets --locked

test:
  cargo test

lint:
  cargo clippy --workspace --all-targets --all-features -- --deny warnings
