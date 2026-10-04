#!/usr/bin/env bash
set -euo pipefail

uuid="promptheus@promptheus.desktop"
source_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$uuid"
target_dir="$HOME/.local/share/gnome-shell/extensions"

mkdir -p "$target_dir"
ln -sfn "$source_dir" "$target_dir/$uuid"

echo "Linked $target_dir/$uuid -> $source_dir"
echo "Log out and back in, then run: gnome-extensions enable $uuid"
