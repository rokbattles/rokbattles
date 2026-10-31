# Repository Guidelines

## Project Structure & Module Organization

ROK Battles combines a Rust workspace with pnpm frontend packages.

- `crates/apps/`: API, ingress, processor, scheduled jobs, networking services, and Tauri desktop backend.
- `crates/common/`: shared codecs, mail processors, SDKs, and WASM libraries. Rust source and inline tests live in each crate’s `src/`.
- `packages/site/`: Next.js site; `packages/desktop-client/`: React/Vite desktop frontend. Assets live in package `public/` directories and `src/assets/` where present.
- `datasets/`: game data; `infra/`: deployment configuration; `.github/workflows/`: CI.

## Build, Test, and Development Commands

Use Rust 1.99.1+, Node.js 26+, and the pnpm version pinned in `package.json`. Backend services require MongoDB.

- `pnpm install --frozen-lockfile`: install frontend dependencies.
- `pnpm -F @rokbattles/site dev`: start the Next.js site.
- Before `pnpm -F @rokbattles/site build`, run that package’s `generate:wasm`, `sync:legal`, and `generate:datasets` scripts. Install Rust’s `wasm32-unknown-unknown` target first; legal synchronization requires network access.
- `pnpm -F @rokbattles/desktop-client build`: type-check and build the desktop frontend.
- `pnpm -F @rokbattles/desktop-client tauri dev`: run the desktop app with native prerequisites installed.
- `cargo build --workspace --all-targets`: build Rust targets.
- `cargo test --workspace --all-features`: run Rust tests; use `cargo test -p <crate>` for focused checks.

## Coding Style & Naming Conventions

TypeScript uses Biome: two spaces, double quotes, semicolons, and a 100-column width. Run `pnpm format` to apply fixes or `pnpm exec biome ci .` to check. Follow nearby filename conventions; use PascalCase React components and `use`-prefixed hooks.

Rust uses edition 2024, four-space indentation, and snake_case functions/modules. Check with `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

## Testing Guidelines

Add behavior-focused Rust tests in `#[cfg(test)] mod tests`, using descriptive snake_case names and `#[test]` or `#[tokio::test]`. CI reports `cargo llvm-cov` coverage without a numeric minimum. Frontend packages have no test scripts; run affected builds and manually verify UI changes.

## Commit & Pull Request Guidelines

Follow history’s Conventional Commit style: `fix(desktop):`, `refactor(site):`, or `chore:`. Keep changes focused. PRs should explain behavior changes, link relevant issues, list validation, and include screenshots for UI changes.

## Configuration

Use package-specific `.env.example` files; keep credentials out of commits. Regenerate derived assets through package scripts.
