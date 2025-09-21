# Repository Guidelines

## Project Structure & Module Organization
Workspace splits into `crates/` (Rust services, processors, CLI tools) and `apps/` (user-facing UIs). Each crate keeps sources in `src/` with integration suites in `tests/`. `apps/rokbattles-site` hosts the Next.js frontend; it consumes shared assets from `public/` and feeds the Tauri wrapper in `crates/rokbattles-tauri/src-tauri`. Datasets for local experiments live in `datasets/`, while `samples/` carries mail fixtures referenced by tests.

## Build, Test, and Development Commands
- `pnpm install --frozen-lockfile` syncs JavaScript dependencies across the workspace.
- `pnpm dev:site` serves Next.js with Turbo; `pnpm dev:tauri` boots the desktop shell.
- `pnpm -F @rokbattles/site build` and `pnpm -F @rokbattles/tauri build` validate production bundles.
- `cargo build --workspace --all-targets` (repeat with `--no-default-features`) verifies Rust compilation.
- `cargo test --workspace --all-features` mirrors CI; follow with the no-default-features variant when feature flags change.
- `biome ci .`, `pnpm format`, `cargo fmt --all`, and `cargo clippy --workspace --all-targets` must be clean before pushing.

## Coding Style & Naming Conventions
Rust code is formatted with `cargo fmt` and must compile without clippy warnings (`RUSTFLAGS=-Dwarnings`). Use `snake_case` for functions/modules, `PascalCase` for types, and prefer explicit imports. TypeScript follows the Biome profile: 2-space indentation, double quotes, trailing commas, and organized imports. Keep Next.js route folders lowercase and compose Tailwind classes through `clsx` helpers rather than inline string concatenation.

## Testing Guidelines
CI runs `cargo test` on stable and nightly across platforms; keep parity locally with `cargo test --workspace --all-features` plus the `--no-default-features` sweep. Place integration cases under `crates/<name>/tests/` and favor table-driven datasets pulled from `samples/`. Frontend work must at least pass `pnpm -F @rokbattles/site build`; document manual smoke steps or screenshots in the PR until automated UI tests land.

## Commit & Pull Request Guidelines
Use Conventional Commit prefixes (`feat:`, `fix:`, `refactor:`) with optional scopes such as `feat(mail-helper):`. Reference issues in the subject as `(#27)` like existing history. PRs need a crisp summary, linked issues, verification command list, and UI captures when visuals change. Request review only after CI (lint, build, test) returns green.

## Environment & Configuration Notes
Develop with Node.js 22.x, pnpm 10.15.1, and Rust 1.89+; install Clang plus the GTK/WebKit packages noted in `.github/workflows/ci.yml` if you're on macOS or Linux. Nightly toolchains are optional locally but help reproduce CI edge cases (`rustup toolchain install nightly`). Keep secrets in ignored `.env.local` files and never commit datasets containing private player information.
