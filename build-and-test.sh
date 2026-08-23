#!/usr/bin/env bash

# Everything CI checks in this repository, in one command.
#
#   ./build-and-test.sh          format, lint, test, and every documented snippet
#   ./build-and-test.sh check    the same thing; the name CI uses
#   ./build-and-test.sh fix      format Rust and C++ in place first
#
# **Half of what runs is in `bin/gate-common.sh`**, of which every repository
# in the organisation carries a byte-identical copy. This file is what this
# repository configures, what only it checks, and the order they run in.
# `xpui-dev` compares the nine copies and runs all nine gates.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${PROJECT_DIR}"

# One crate, at the root. `file_sizes` prunes `target`, so an extracted
# package's `src/` is not read as ours.
SOURCE_ROOTS=(.)

# The simulator is a desktop window: it links SDL2 and builds for no device.
# `LINT_TARGETS` is empty for that reason and not by omission.
TEST_FEATURES=""
HOST_WORKSPACE=1

. bin/gate-common.sh

gates() {
  file_sizes
  every_check_runs
  readmes_warn
  prose_is_compiled
  doc_paths
  commands_resolve
  cpp_snippets_compile
  lint
  test_suite
  doc_tests
  doc_links
}

case "${1:-check}" in
  check)
    run_all "${FORMAT_CHECK[@]}"
    gates
    printf '\nChecks passed.\n'
    ;;
  fix)
    run_all "${FORMAT_FIX[@]}"
    gates
    printf '\nFormatted and checked.\n'
    ;;
  *)
    echo "usage: ./build-and-test.sh [check|fix]" >&2
    exit 2
    ;;
esac
