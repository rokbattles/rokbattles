# Repository Guidelines

## Project Structure & Module Organization
- `apps/`: Frontend and node apps (`rokbattles-platform` Next.js site, `rokbattles-bot` Discord bot, marketing site). Use `pnpm --filter` to scope commands.
- `crates/`: Rust workspace crates (API, processors, CLI helpers, Tauri shell). Workspace root `Cargo.toml` wires shared deps.
- `datasets/` and `samples/`: Reference data and example battle reports used by generators and tests.
- `infra/` and `docker-compose.yml`: Local service scaffolding; prefer reproducible containers over ad-hoc installs.
- `legal/`, licenses, and root configs (`biome.jsonc`, `pnpm-workspace.yaml`) live at the repo root.

## Build, Test, and Development Commands
- Install deps once with `pnpm install` (enforced via `npx only-allow pnpm`).
- Rust: `cargo build --workspace` and `cargo test --workspace` from the repo root.
- Web app: `pnpm --filter @rokbattles/platform dev` for local dev, `pnpm --filter @rokbattles/platform build` for CI-style builds.
- Bot: `pnpm --filter @rokbattles/bot build` then `pnpm --filter @rokbattles/bot start` (after registering commands as needed).
- Tauri shell: `cargo run -p rokbattles-tauri` from `crates/rokbattles-tauri/src-tauri`.
- Dataset helpers: `pnpm --filter @rokbattles/platform generate:datasets` or `pnpm --filter @rokbattles/bot generate:datasets`.

## Coding Style & Naming Conventions
- TypeScript/JS: Run `pnpm format` (Biome) before pushing; keep React components in `PascalCase`, helpers in `camelCase`, files in `kebab-case`.
- Rust: Use `cargo fmt` and `cargo clippy --workspace -- -D warnings`; prefer `snake_case` for modules and functions and `PascalCase` for types.
- Target Rust 1.92+ and Node.js 24.x; keep imports sorted by Biome/Rustfmt defaults.

## Testing Guidelines
- Primary coverage uses `cargo test --workspace`; place Rust unit tests beside modules with `#[cfg(test)]`.
- Add integration or CLI tests when touching processors/APIs; prefer realistic sample data from `samples/`.
- No frontend/bot test harness is shipped; if you add one, co-locate specs with source and wire the command into the package’s `scripts`.

## Commit & Pull Request Guidelines
- Follow the existing pattern: `<type>: <concise summary> (#<issue|PR>)` (e.g., `chore: update to node 24 (#160)`).
- PRs should describe scope, include relevant logs or screenshots for UI/API changes, and link issues.
- Note breaking changes explicitly and list the commands you ran (build/test) in the PR body.

## Security & Configuration Tips
- Do not commit secrets; keep environment values in local `.env` files or container env vars, and add new secrets to ignored lists when needed.
- Align with the dual MIT/Apache-2.0 licensing noted in `LICENSE-MIT` and `LICENSE-APACHE`.
