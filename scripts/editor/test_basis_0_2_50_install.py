import importlib.util,os,tempfile,hashlib,json,subprocess
from pathlib import Path
p=Path(__file__).with_name('install_basis_0_2_50.py')
s=importlib.util.spec_from_file_location('candidate',p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
with tempfile.TemporaryDirectory(prefix='Editor Port Test ') as tmp:
 d=Path(tmp);src=d/'source';target=d/'build';dest=d/'editor';old=d/'old';old.mkdir();(old/'openomsi').write_bytes(b'untouched')
 for x in [src/'assets',src/'docs',target/'release']:x.mkdir(parents=True)
 (target/'release/openomsi').write_text('#!/bin/sh\nprintf "%s\\n" "$OMSI_CONTENT" "$OMSI_NO_UPDATE" "$PWD" "$1"\n')
 (target/'release/openomsi-launcher-app').write_bytes(b'launcher')
 (target/'release/libsteam_api.so').write_bytes(b'runtime');(src/'assets/test').write_bytes(b'asset');(src/'docs/test.pdf').write_bytes(b'pdf')
 (src/'LICENSE').write_bytes(b'license');(src/'README.md').write_bytes(b'readme')
 (src/'SOURCE-SHA256.json').write_text(json.dumps({'version':m.VERSION,'files':{'assets/test':hashlib.sha256(b'asset').hexdigest()}}))
 m.verify_package(src);os.environ['CARGO_TARGET_DIR']=str(target);m.install(src,dest)
 assert 'openOMSI-Basis 0.2.50' in (dest/'VERSION-EDITOR.txt').read_text()
 assert (dest/'docs/LICENSE').read_bytes()==b'license'
 assert (dest/'docs/README.md').read_bytes()==b'readme'
 assert not (dest/'LICENSE').exists() and not (dest/'README.md').exists()
 game=d/'Inhalte & Karten';game.mkdir()
 result=subprocess.check_output(['bash',str(dest/'start-editor.sh'),str(game),'argument mit leerzeichen'],text=True).splitlines()
 assert result==[str(game),'1',str(game),'argument mit leerzeichen'],result
 try:m.install(src,dest)
 except ValueError:pass
 else:raise AssertionError('existing installation overwritten')
 assert (old/'openomsi').read_bytes()==b'untouched'
 (target/'release/libsteam_api.so').unlink()
 try:m.install(src,d/'incomplete')
 except ValueError:pass
 else:raise AssertionError('incomplete installation published')
 assert not (d/'incomplete').exists()
 (src/'assets/test').write_bytes(b'changed')
 try:m.verify_package(src)
 except ValueError:pass
 else:raise AssertionError('modified source accepted')
print('Port installer passed: separate complete installation, launcher arguments, overwrite protection, missing runtime and checksum rejection.')
