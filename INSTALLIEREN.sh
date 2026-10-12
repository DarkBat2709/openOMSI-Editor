#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
python3 scripts/editor/install_basis_0_2_38.py "$@" 2>&1 | tee update-basis-0.2.38.log
