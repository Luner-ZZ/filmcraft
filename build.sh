#!/usr/bin/env bash
# Universal 1-Click Builder for macOS and Linux
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

if [[ "$(uname -s)" == "Darwin" ]]; then
  exec "$DIR/build-macos.command" "$@"
else
  exec "$DIR/build-linux.sh" "$@"
fi
