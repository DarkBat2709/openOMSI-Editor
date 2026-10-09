#!/usr/bin/env python3
"""Check editor locale keys and exercise the real UI translator with rustc (no extra Python packages)."""
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def read_locale(path):
    entries = {}
    key = None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith('"') and line.endswith('":'):
            key = json.loads(line[:-1])
            if key in entries:
                raise AssertionError(f"Duplicate translation key: {key}")
            entries[key] = {}
        elif key is not None and re.match(r"  [a-z-]+: ", line):
            lang, value = line.strip().split(": ", 1)
            entries[key][lang] = json.loads(value)
    return entries


def rust(s):
    # JSON escapes are Rust-compatible here, with literal Unicode retained.
    return json.dumps(s, ensure_ascii=False)


def main():
    editor = read_locale(ROOT / "crates/omsi-app/locales/editor.yml")
    upstream = read_locale(ROOT / "crates/omsi-app/locales/app.yml")
    assert not editor.keys() & upstream.keys(), "Editor overrides an upstream key"
    holes = re.compile(r"\{([^{}]*)\}")
    samples = []
    for key, languages in editor.items():
        assert "de" in languages, key
        de = languages["de"]
        assert holes.findall(key) == holes.findall(de), f"Changed format arguments: {key}"
        names = {}
        pos = [0]
        def sample(m):
            name = m[1].split(":")[0]
            if not name:
                pos[0] += 1
                return f"V{pos[0]}"
            return names.setdefault(name, f"N{len(names) + 1}")
        en_sample = holes.sub(sample, key)
        pos[0] = 0
        de_sample = holes.sub(sample, de)
        samples.append((en_sample, de_sample))
    table = upstream | editor
    arms = []
    for key, langs in table.items():
        for lang in ("de", "fr"):
            if lang in langs:
                arms.append(f"({rust(lang)}, {rust(key)}, {rust(langs[lang])}),")
    # Use the real crate source, not a reimplementation of template matching.
    program = f'''#[path = {rust(str(ROOT / "crates/omsi-ui/src/i18n.rs"))}]
mod i18n;
fn lookup(lang: &str, key: &str) -> Option<String> {{
    const TABLE: &[(&str, &str, &str)] = &[{''.join(arms)}];
    TABLE.iter().find(|(l, k, _)| *l == lang && *k == key).map(|(_, _, v)| (*v).to_owned())
}}
fn main() {{
    i18n::set_lookup(lookup);
    i18n::set_templates([{','.join(rust(k) + '.to_owned()' for k in upstream if '{' in k)}]);
    i18n::set_explicit_templates([{','.join(rust(k) + '.to_owned()' for k in editor if '{' in k)}]);
    let samples = [{','.join('(' + rust(a) + ',' + rust(b) + ')' for a,b in samples)}];
    // Reuse the same strings while changing language: cached statuses must switch too.
    for lang in ["de", "en", "de", "en"] {{
        i18n::set_language(lang);
        for (en, de) in samples {{
            let want = if lang == "de" {{ de }} else {{ en }};
            assert_eq!(i18n::tr(en), want, "{{lang}}: {{en}}");
            if lang == "de" {{ assert_eq!(i18n::tr(de), de, "Double translation: {{de}}"); }}
        }}
    }}
    i18n::set_language("de");
    assert_eq!(i18n::tr("Sort: Category"), "Sort: Kategorie");
    assert_eq!(i18n::tr("Not created: Invalid tile height"), "Nicht angelegt: Ungültige Tile-Höhe");
    assert_eq!(i18n::tr("Road.sli"), "Road.sli");
    assert_eq!(i18n::tr("12.52"), "12.52");
    i18n::set_language("fr");
    assert_eq!(i18n::tr("Close"), lookup("fr", "Close").unwrap());
    assert_eq!(i18n::tr("Crop U from"), "Crop U from");
    println!("Editor locale checks passed: {{}} keys; live DE/EN switch, nested messages and fallback", samples.len());
}}
'''
    with tempfile.TemporaryDirectory(prefix="openomsi-editor-i18n-") as tmp:
        source = Path(tmp) / "check.rs"
        binary = Path(tmp) / ("check.exe" if os.name == "nt" else "check")
        source.write_text(program, encoding="utf-8")
        compiler = os.environ.get("RUSTC", "rustc")
        subprocess.run([compiler, *shlex.split(os.environ.get("RUSTFLAGS", "")), "--edition=2021", "-A", "dead_code", str(source), "-o", str(binary)], check=True)
        binary.chmod(binary.stat().st_mode | 0o111)
        subprocess.run([str(binary)], check=True)


if __name__ == "__main__":
    main()
