#! /bin/bash

set -e
cd "$CARGO_MANIFEST_DIR"/examples/termination/
"$RCC_EXECUTABLE" task script -- true
"$RCC_EXECUTABLE" ht export linux
"$RCC_EXECUTABLE" configuration cleanup --all
