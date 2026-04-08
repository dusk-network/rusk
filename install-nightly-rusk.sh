#!/usr/bin/env bash

### This script finds the nightly version of Rust that corresponds to a given stable version,
### and installs it with rustup.
###
### This is useful for building Rusk with the same nightly version as the one used for the
### latest stable release, which is important for reproducibility and debugging.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLCHAIN_FILE="$SCRIPT_DIR/rust-toolchain.toml"

get_version_from_rust_toolchain() {
  [ -f "$TOOLCHAIN_FILE" ] || return 1

  sed -n '
    /^[[:space:]]*channel[[:space:]]*=/ {
      s/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p
      q
    }
  ' "$TOOLCHAIN_FILE"
}

normalize_to_stable_base() {
  local raw="$1"
  local major minor

  case "$raw" in
    [0-9]*.[0-9]*.[0-9]*)
      major=${raw%%.*}
      raw=${raw#*.}
      minor=${raw%%.*}
      ;;
    [0-9]*.[0-9]*)
      major=${raw%%.*}
      minor=${raw#*.}
      ;;
    *)
      echo "Unsupported Rust version format: $1" >&2
      return 1
      ;;
  esac

  printf '%s.%s.0\n' "$major" "$minor"
}

RAW_VERSION="${1:-}"

if [ -z "$RAW_VERSION" ]; then
  RAW_VERSION="$(get_version_from_rust_toolchain || true)"
fi

[ -n "$RAW_VERSION" ] || {
  echo "No Rust version provided and could not read channel from rust-toolchain.toml" >&2
  exit 1
}

VERSION="$(normalize_to_stable_base "$RAW_VERSION")"

echo "Requested version: $RAW_VERSION"
echo "Normalized version: $VERSION"

CURRENT_VERSION=$(rustup run nightly-rusk rustc --version 2>/dev/null || true)
if printf '%s\n' "$CURRENT_VERSION" | grep -q "rustc ${VERSION}-nightly"; then
  echo "nightly-rusk already matches rustc ${VERSION}-nightly"
  exit 0
fi

DATE=$(
  curl -fsS https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md \
    | grep -A1 "Version $VERSION" \
    | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' \
    | head -n1
)

[ -n "$DATE" ] || { echo "Could not find release date for $VERSION" >&2; exit 1; }

echo "Release date for $VERSION: $DATE"

add_days() {
  local base="$1"
  local offset="$2"

  if date -d "$base" +%F >/dev/null 2>&1; then
    date -d "$base $offset day" +%F
  else
    date -v"${offset}"d -j -f "%Y-%m-%d" "$base" +%F
  fi
}

# Start around the expected beta-branch date (~6 weeks before stable)
SEARCH_START=$(add_days "$DATE" "-42")
echo "Search start date: $SEARCH_START"

for i in {0..30}; do
  TRY_DATE=$(add_days "$SEARCH_START" "-$i")
  echo "Trying $TRY_DATE"

  TOML=$(curl -fsS "https://static.rust-lang.org/dist/$TRY_DATE/channel-rust-nightly.toml" || true)
  [ -z "$TOML" ] && continue

  NIGHTLY_VERSION=$(
    printf '%s\n' "$TOML" | sed -n '
      /^\[pkg\.rust\]/,/^\[/ {
        s/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p
      }
    ' | head -n1
  )

  [ -n "$NIGHTLY_VERSION" ] || continue

  BASE_VERSION=${NIGHTLY_VERSION%%-nightly*}

  if [ "$BASE_VERSION" = "$VERSION" ]; then
    echo "Found matching nightly: $TRY_DATE ($NIGHTLY_VERSION)"
    rustup install "nightly-$TRY_DATE"
    rustup component add rust-src --toolchain "nightly-$TRY_DATE"
    rustup component add clippy --toolchain "nightly-$TRY_DATE"
    rustup toolchain remove nightly-rusk >/dev/null 2>&1 || true
    rustup toolchain link nightly-rusk \
      "$(dirname "$(dirname "$(rustup which --toolchain "nightly-$TRY_DATE" rustc)")")"
    exit 0
  fi
done

echo "No nightly found in range"
exit 1