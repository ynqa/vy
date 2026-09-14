#!/usr/bin/env bash
set -euo pipefail

# Requires Go, ttyd, and ffmpeg. Run cargo build --release --locked first.
# Usage: bash docs/demos/record.sh docs/demos/explore/search/matches.tape
cd "$(dirname "$0")/../.."
demo_cache="$PWD/docs/demos/.work"
demo_patch="$PWD/docs/demos/shared/vhs-sync.patch"
demo_vhs="$demo_cache/vhs"
mkdir -p "$demo_cache"

# VHS 0.11's canvas terminal ignores synchronized output (DEC mode 2026).
# Apply a recording-only compatibility fix to avoid capturing partial redraws.
if [[ ! -x "$demo_vhs" || "$demo_patch" -nt "$demo_vhs" ]]; then
  go mod download github.com/charmbracelet/vhs@v0.11.0
  demo_source=$(mktemp -d "$demo_cache/vhs-source.XXXXXX")
  trap 'rm -rf "$demo_source"' EXIT
  cp -R "$(go env GOMODCACHE)/github.com/charmbracelet/vhs@v0.11.0/." "$demo_source"
  chmod -R u+w "$demo_source"
  patch -d "$demo_source" -p1 < "$demo_patch"
  (cd "$demo_source" && go build -o "$demo_vhs" .)
  rm -rf "$demo_source"
  trap - EXIT
fi

exec "$demo_vhs" "$@"
