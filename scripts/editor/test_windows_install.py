"""Filesystem contract tests; also run on Windows to exercise replacement semantics."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('editor_windows', Path(__file__).with_name('windows_editor.py'))
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


class InstallTests(unittest.TestCase):
    def test_separate_install_update_backup_and_unicode_paths(self):
        with tempfile.TemporaryDirectory(prefix='editor Straße ') as tmp:
            root = Path(tmp)
            game, build, assets, editor = [root / name for name in ('Spiel & Karten', 'build', 'assets', 'Editor')]
            for folder in (game, build, assets / 'steam_redist'):
                folder.mkdir(parents=True)
            (game / 'openomsi.exe').write_bytes(b'official')
            (game / 'dxcompiler.dll').write_bytes(b'dxc')
            (assets / 'steam_redist/steam_api64.dll').write_bytes(b'matching steam')
            binary = build / 'openomsi.exe'
            binary.write_bytes(b'editor 1')
            helper.install(binary, game, editor, assets)
            binary.write_bytes(b'editor 2')
            helper.install(binary, game, editor, assets)
            self.assertEqual((game / 'openomsi.exe').read_bytes(), b'official')
            self.assertEqual((editor / 'openomsi.exe').read_bytes(), b'editor 2')
            self.assertEqual(next(editor.glob('openomsi.exe.before-editor-*')).read_bytes(), b'editor 1')
            self.assertEqual((editor / 'steam_api64.dll').read_bytes(), b'matching steam')
            self.assertEqual(helper.json.loads((editor / 'editor-config.json').read_text(encoding='utf-8'))['content'], str(game.resolve()))
            for unsafe in (game, game / 'editor', root):
                with self.assertRaises(ValueError):
                    helper.install(binary, game, unsafe, assets)
            unrelated = root / 'unrelated'
            unrelated.mkdir()
            (unrelated / 'keep.txt').write_text('keep')
            with self.assertRaises(ValueError):
                helper.install(binary, game, unrelated, assets)
            self.assertEqual((unrelated / 'keep.txt').read_text(), 'keep')


if __name__ == '__main__':
    unittest.main()
