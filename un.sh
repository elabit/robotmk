#! /bin/bash

set -e

CARGO_MANIFEST_DIR="$1"
RCC_EXECUTABLE="$2"
TERMINATION_EXECUTABLE="$3"

cd "$CARGO_MANIFEST_DIR"/examples/termination/
"$RCC_EXECUTABLE" holotree import hololib.zip
"$TERMINATION_EXECUTABLE" -- "$RCC_EXECUTABLE"
