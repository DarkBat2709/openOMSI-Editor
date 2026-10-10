#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
python3 scripts/editor/install_0_7_21.py "$@" 2>&1 | tee update-0.7.21-pre.log
