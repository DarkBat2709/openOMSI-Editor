import tempfile
import unittest
from pathlib import Path
from check_map_compatibility import scan_map, scan_text

class CompatibilityTests(unittest.TestCase):
    def test_standard_rules_and_comments_are_not_extensions(self):
        self.assertEqual(scan_text('[rule]\n0\nspeed\n30\n; [openomsi_editor_deleted]\n'), [])

    def test_extensions_are_reported_by_type_and_line(self):
        text = '[rule]\n0\neditor_lane_shape\n1\n[openomsi_profile_transition]\n[openomsi_editor_deleted]\n'
        self.assertEqual(scan_text(text), [
            {'line': 1, 'kind': 'ai_path_rule'},
            {'line': 5, 'kind': 'profile_transition'},
            {'line': 6, 'kind': 'deleted_spline'}])

    def test_generated_path_rules_are_also_editor_specific(self):
        self.assertEqual(len(scan_text('[rule]\n0\neditor_surface_path\n1\n[rule]\n0\neditor_bus_only\n1\n')), 2)

    def test_utf16_chrono_and_read_only(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'global.cfg').write_text('[name]\nTest\n')
            path = root / 'Chrono' / 'test' / 'tile_0_0.map'
            path.parent.mkdir(parents=True)
            content = '[openomsi_editor_deleted]\n'.encode('utf-16')
            path.write_bytes(content)
            report = scan_map(root)
            self.assertEqual(report['tiles_checked'], 1)
            self.assertEqual(report['findings'][0]['file'], 'Chrono/test/tile_0_0.map')
            self.assertEqual(path.read_bytes(), content)

if __name__ == '__main__':
    unittest.main()
