#!/usr/bin/env python3
"""Read-only inventory of editor extensions unsupported by stock openOMSI 0.2.50.

A clean result concerns these extensions only; it does not certify assets or gameplay.
"""
import argparse
import json
from pathlib import Path

TAGS = {'[openomsi_profile_transition]': 'profile_transition',
        '[openomsi_editor_deleted]': 'deleted_spline'}

def scan_text(text):
    lines = [line.strip() for line in text.splitlines()]
    found = []
    for i, line in enumerate(lines):
        if line in TAGS:
            found.append({'line': i + 1, 'kind': TAGS[line]})
        elif line == '[rule]':
            # A rule starts with the path index and then the rule kind.
            values = [v for v in lines[i + 1:i + 4] if v]
            if len(values) >= 2 and values[1].startswith('editor_'):
                found.append({'line': i + 1, 'kind': 'ai_path_rule'})
    return found

def scan_map(root):
    root = Path(root)
    if not root.is_dir() or not (root / 'global.cfg').is_file():
        raise ValueError('Choose a map directory containing global.cfg')
    files = sorted([*root.glob('*.map'), *(root / 'Chrono').rglob('*.map')])
    findings = []
    for path in files:
        raw = path.read_bytes()
        text = raw.decode('utf-16') if raw.startswith((b'\xff\xfe', b'\xfe\xff')) else raw.decode('utf-8-sig', errors='surrogateescape')
        for item in scan_text(text):
            findings.append({'file': path.relative_to(root).as_posix(), **item})
    return {'stock_version': '0.2.50', 'tiles_checked': len(files), 'findings': findings,
            'scope': 'Known editor extensions only; assets and gameplay need separate tests.'}

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('map_directory')
    args = parser.parse_args()
    print(json.dumps(scan_map(args.map_directory), ensure_ascii=False, indent=2))
