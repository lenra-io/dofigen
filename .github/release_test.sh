#!/bin/bash

# Unit tests for the `get_tag` function in release.sh.
# Run with: bash .github/release_test.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=release.sh
source "${SCRIPT_DIR}/release.sh"
# release.sh enables `set -xe` for its build run; reset to sane options for testing.
set +xe

failures=0

# assert_tag <name> <version> <image> <prefix> <expected>
assert_tag() {
  local name="$1" version="$2" image="$3" prefix="$4" expected="$5"
  tag=""
  get_tag "$version" "$image" "$prefix"
  if [[ "$tag" != "$expected" ]]; then
    echo "FAIL: $name"
    echo "  expected: $expected"
    echo "  actual:   $tag"
    failures=$((failures + 1))
  else
    echo "PASS: $name"
  fi
}

# assert_fails <name> <version>
assert_fails() {
  local name="$1" version="$2"
  if get_tag "$version" "lenra/dofigen" "" >/dev/null 2>&1; then
    echo "FAIL: $name (expected a non-zero exit code)"
    failures=$((failures + 1))
  else
    echo "PASS: $name"
  fi
}

# --- Base binary tags (no prefix) ---------------------------------------------

assert_tag "stable release, no prefix" "2.1.3" "lenra/dofigen" "" \
  "--tag lenra/dofigen:latest --tag lenra/dofigen:2 --tag lenra/dofigen:2.1 --tag lenra/dofigen:2.1.3"

assert_tag "leading v is stripped" "v2.1.3" "lenra/dofigen" "" \
  "--tag lenra/dofigen:latest --tag lenra/dofigen:2 --tag lenra/dofigen:2.1 --tag lenra/dofigen:2.1.3"

assert_tag "prerelease channel, no prefix" "2.1.3-beta.1" "lenra/dofigen" "" \
  "--tag lenra/dofigen:2.1.3-beta.1 --tag lenra/dofigen:beta --tag lenra/dofigen:2-beta --tag lenra/dofigen:2.1-beta --tag lenra/dofigen:2.1.3-beta"

# --- Frontend image tags (syntax- prefix) -------------------------------------

# The "latest" equivalent for the frontend is "syntax" (prefix without the dash).
assert_tag "stable release, syntax- prefix" "2.1.3" "lenra/dofigen" "syntax-" \
  "--tag lenra/dofigen:syntax --tag lenra/dofigen:syntax-2 --tag lenra/dofigen:syntax-2.1 --tag lenra/dofigen:syntax-2.1.3"

assert_tag "single digit major, syntax- prefix" "2.0.0" "lenra/dofigen" "syntax-" \
  "--tag lenra/dofigen:syntax --tag lenra/dofigen:syntax-2 --tag lenra/dofigen:syntax-2.0 --tag lenra/dofigen:syntax-2.0.0"

assert_tag "prerelease channel, syntax- prefix" "2.1.3-beta.1" "lenra/dofigen" "syntax-" \
  "--tag lenra/dofigen:syntax-2.1.3-beta.1 --tag lenra/dofigen:syntax-beta --tag lenra/dofigen:syntax-2-beta --tag lenra/dofigen:syntax-2.1-beta --tag lenra/dofigen:syntax-2.1.3-beta"

# --- Invalid versions ---------------------------------------------------------

assert_fails "rejects a non-version string" "not-a-version"
assert_fails "rejects an empty version" ""

if [[ $failures -gt 0 ]]; then
  echo ""
  echo "$failures test(s) failed"
  exit 1
fi

echo ""
echo "All tests passed"
