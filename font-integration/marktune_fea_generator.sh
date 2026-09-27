#!/usr/bin/env bash
# Usage: python3 marktune_fea_generator.py <NUM_ROWS> <STEP_PERCENT> <UPM> [output.fea]

echo "Usage: python3 marktune_fea_generator.py <NUM_ROWS> <STEP_PERCENT> <UPM> [output.fea]"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PY_SCRIPT="$SCRIPT_DIR/marktune_fea_generator.py"

cd "$SCRIPT_DIR"

python3 "$PY_SCRIPT" 24 4 1000

read -r -p "Press enter to continue" < /dev/tty
