#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"
app_id=com.rokbattles.rokbattles
build_root="$repo_root/target/flatpak"
arch=$(flatpak --default-arch)
version=$(node -p 'JSON.parse(require("node:fs").readFileSync("crates/apps/rokbattles-desktop/tauri.conf.json", "utf8")).version')

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user --noninteractive -y flathub \
  org.gnome.Platform//50 org.gnome.Sdk//50

# Tauri builds the frontend and generates the desktop entry and icons in the deb.
CARGO_TARGET_DIR="$build_root/cargo" pnpm --filter @rokbattles/desktop-client tauri build \
  --ci --bundles deb \
  --config '{"identifier":"com.rokbattles.rokbattles","bundle":{"createUpdaterArtifacts":false}}' \
  -- --locked
cp "$build_root"/cargo/release/bundle/deb/*_"$version"_"$(dpkg --print-architecture)".deb \
  "$build_root/app.deb"

flatpak-builder --user --force-clean \
  --state-dir="$build_root/cache" --repo="$build_root/repo" \
  "$build_root/build" .github/flatpak/com.rokbattles.rokbattles.yml

mkdir -p dist
bundle="$repo_root/dist/$app_id-$version-$arch.flatpak"
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  "$build_root/repo" "$bundle" "$app_id"
printf '\nBuilt %s\n' "$bundle"
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  printf 'bundle=%s\n' "$bundle" >> "$GITHUB_OUTPUT"
fi
