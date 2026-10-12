"""Exercise candidate installation with fake build outputs, never real game data."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('candidate', Path(__file__).with_name('update_0_2_27.py'))
candidate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(candidate)


class CandidateInstallTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='Editor Test ')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.source = self.root / 'source'
        for folder in ('target/release', 'assets', 'docs'):
            (self.source / folder).mkdir(parents=True)
        (self.source / 'target/release/openomsi').write_bytes(b'candidate')
        (self.source / 'target/release/libsteam_api.so').write_bytes(b'runtime')
        (self.source / 'assets/icon.txt').write_text('asset')
        (self.source / 'docs/guide.txt').write_text('guide')
        self.destination = self.root / 'new editor'
        self.old = self.root / 'old editor'
        self.old.mkdir()
        (self.old / 'openomsi').write_bytes(b'previous working version')

    def test_complete_install_is_separate_and_launcher_handles_spaces(self):
        with patch.object(candidate, 'git', return_value='a' * 40):
            candidate.install(self.source, self.destination)
        self.assertEqual((self.old / 'openomsi').read_bytes(), b'previous working version')
        self.assertEqual((self.destination / 'libsteam_api.so').read_bytes(), b'runtime')
        self.assertTrue((self.destination / 'docs/guide.txt').is_file())
        # Run the actual launcher and capture its working directory and environment.
        executable = self.destination / 'openomsi'
        executable.write_text('#!/bin/sh\nprintf "%s\\n" "$OMSI_CONTENT" "$OMSI_NO_UPDATE" "$PWD" "$1"\n')
        executable.chmod(0o755)
        game = self.root / 'Spiel & Karten'
        game.mkdir()
        output = candidate.subprocess.check_output(
            ['bash', str(self.destination / 'start-editor.sh'), str(game), 'argument with spaces'], text=True)
        self.assertEqual(output.splitlines(), [str(game), '1', str(game), 'argument with spaces'])

    def test_missing_runtime_does_not_publish_a_partial_editor(self):
        (self.source / 'target/release/libsteam_api.so').unlink()
        with self.assertRaises(ValueError):
            candidate.install(self.source, self.destination)
        self.assertFalse(self.destination.exists())
        self.assertEqual((self.old / 'openomsi').read_bytes(), b'previous working version')

    def test_existing_destination_is_never_overwritten(self):
        with self.assertRaises(ValueError):
            candidate.install(self.source, self.old)
        self.assertEqual((self.old / 'openomsi').read_bytes(), b'previous working version')

    def test_failed_build_keeps_original_branch_and_never_installs(self):
        source = self.source
        def call(*args):
            return candidate.subprocess.check_output(
                ['git', '-C', str(source), *args], text=True,
                stderr=candidate.subprocess.DEVNULL).strip()
        call('init', '-b', 'editor-preview')
        call('config', 'user.name', 'Installer test')
        call('config', 'user.email', 'test@example.invalid')
        call('add', '.')
        call('commit', '-m', 'working editor')
        base = call('rev-parse', 'HEAD')
        call('switch', '-c', candidate.BRANCH)
        (source / 'candidate.txt').write_text('new candidate')
        call('add', '.')
        call('commit', '-m', 'candidate')
        sha = call('rev-parse', 'HEAD')
        package = self.root / 'package'
        package.mkdir()
        call('bundle', 'create', str(package / 'editor-update.bundle'), candidate.BRANCH, '^' + base)
        (package / 'candidate.json').write_text(candidate.json.dumps({'commit': sha}))
        call('switch', 'editor-preview')
        call('branch', '-D', candidate.BRANCH)
        commands = []
        real_run = candidate.run
        def run(*args, **kwargs):
            commands.append(tuple(map(str, args)))
            if args[0] == 'git':
                return real_run(*args, **kwargs)
            if args[:2] == ('cargo', 'test'):
                raise candidate.subprocess.CalledProcessError(1, args)
        with patch.object(candidate, 'BASE', base), \
             patch.object(candidate, '__file__', str(package / 'update.py')), \
             patch.object(candidate.sys, 'argv', ['update.py', str(source)]), \
             patch.object(candidate.shutil, 'which', return_value='/test/tool'), \
             patch.object(candidate, 'run', side_effect=run), \
             patch.object(candidate, 'install') as install:
            with self.assertRaises(candidate.subprocess.CalledProcessError):
                candidate.main()
            install.assert_not_called()
        self.assertEqual(call('rev-parse', 'HEAD'), base)
        self.assertEqual(call('branch', '--show-current'), 'editor-preview')
        self.assertFalse(call('status', '--porcelain'))
        self.assertFalse(any(cmd[:2] == ('cargo', 'build') for cmd in commands))


if __name__ == '__main__':
    unittest.main()
