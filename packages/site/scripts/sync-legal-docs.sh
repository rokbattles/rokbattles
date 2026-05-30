#!/usr/bin/env bash
set -euo pipefail

repo_url="https://github.com/rokbattles/legal.git"
branch="main"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
site_dir="$(cd -- "${script_dir}/.." && pwd)"
destination="${site_dir}/legal"

if [[ ! -e "${destination}" ]]; then
  git clone --branch "${branch}" "${repo_url}" "${destination}"
  exit 0
fi

if [[ ! -d "${destination}/.git" ]]; then
  echo "${destination} already exists, but it is not a git repository." >&2
  exit 1
fi

git -C "${destination}" remote set-url origin "${repo_url}"
git -C "${destination}" fetch --prune origin
git -C "${destination}" checkout "${branch}"
git -C "${destination}" pull --ff-only origin "${branch}"
