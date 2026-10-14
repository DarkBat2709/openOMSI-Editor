"""Portable package contracts, including clean extraction and missing runtime rejection."""
import hashlib
from pathlib import Path
import tempfile
import unittest
import zipfile
from package_windows import package, NAME, RUNTIME, GUIDES


class PackageTests(unittest.TestCase):
    def test_complete_package_and_missing_dependency(self):
        with tempfile.TemporaryDirectory(prefix='Editor Straße & ') as temp:
            root = Path(temp)
            source, runtime, out = root/'source', root/'runtime', root/'out'
            files = [source/'openomsi.exe', source/'openomsi-launcher-app.exe', source/'LICENSE', source/'assets/steam_redist/steam_api64.dll']
            files += [runtime/name for name in RUNTIME]
            files += [source/'docs'/name for name in GUIDES]
            for p in files:
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_bytes(b'fixture')
            launcher = source/'openomsi-launcher-app.exe'
            launcher.unlink()
            with self.assertRaises(ValueError):
                package(source, source/'openomsi.exe', runtime, out, 'test-sha')
            self.assertFalse(out.exists())
            launcher.write_bytes(b'launcher')
            missing = runtime/RUNTIME[0]
            missing.unlink()
            with self.assertRaises(ValueError):
                package(source, source/'openomsi.exe', runtime, out, 'test-sha')
            self.assertFalse(out.exists())
            missing.write_bytes(b'runtime')
            archive = package(source, source/'openomsi.exe', runtime, out, 'test-sha')
            self.assertEqual((out/(archive.name+'.sha256')).read_text().split()[0], hashlib.sha256(archive.read_bytes()).hexdigest())
            with zipfile.ZipFile(archive) as z:
                self.assertIsNone(z.testzip())
                z.extractall(root/'clean extraction')
            extracted = root/'clean extraction'/NAME
            self.assertIn('test-sha', (extracted/'VERSION-EDITOR.txt').read_text())
            for name in (*RUNTIME, 'steam_api64.dll', 'openomsi.exe', 'openomsi-launcher-app.exe', 'start-editor.cmd'):
                self.assertTrue((extracted/name).is_file(), name)
            self.assertTrue((extracted/'docs'/GUIDES[1]).is_file())
            with self.assertRaises(ValueError):
                package(source, source/'openomsi.exe', runtime, out, 'test-sha')
            self.assertEqual((source/'openomsi.exe').read_bytes(), b'fixture')


if __name__ == '__main__':
    unittest.main()
