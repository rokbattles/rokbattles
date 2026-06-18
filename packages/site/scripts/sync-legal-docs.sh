#!/bin/sh
set -eu

repo_url="https://github.com/rokbattles/legal.git"
legal_commit="fe27a66dcc26d48c6fe0569cb7d10b9cde03ebc5"
script_dir="$(cd "$(dirname "$0")" && pwd)"
site_dir="$(cd "${script_dir}/.." && pwd)"
destination="${site_dir}/legal"

if [ ! -e "${destination}" ]; then
  mkdir -p "${destination}"
  git -C "${destination}" init
  git -C "${destination}" remote add origin "${repo_url}"
fi

if [ ! -d "${destination}/.git" ]; then
  echo "${destination} already exists, but it is not a git repository." >&2
  exit 1
fi

git -C "${destination}" remote set-url origin "${repo_url}"
git -C "${destination}" fetch --depth=1 origin "${legal_commit}"
git -C "${destination}" checkout --detach --force "${legal_commit}"
git -C "${destination}" reset --hard "${legal_commit}"
git -C "${destination}" clean -fdx

current_commit="$(git -C "${destination}" rev-parse HEAD)"
if [ "${current_commit}" != "${legal_commit}" ]; then
  echo "Expected legal docs at ${legal_commit}, got ${current_commit}." >&2
  exit 1
fi
