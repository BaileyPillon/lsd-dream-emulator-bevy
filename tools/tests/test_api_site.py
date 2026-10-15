"""Publication checks using owned HTML, never game files or generated Rust code."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_site
import stage_api


class ApiSiteTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.site = self.root / 'site'
        self.write('index.html', '<a href="api/lsd_core/index.html">Core</a>')

    def write(self, relative, text):
        path = self.site / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding='utf-8')
        return path

    def failures(self, **kwargs):
        return check_site.check(self.site, **kwargs)[1]

    def test_encoded_and_decoded_fragment_ids(self):
        self.write('api/lsd_core/index.html',
                   '<a href="#impl-T%3Cu8%3E">Encoded</a>'
                   '<section id="impl-T%3Cu8%3E"></section>'
                   '<a href="#type%3Cu16%3E">Decoded</a>'
                   '<section id="type&lt;u16&gt;"></section>')
        self.assertEqual(self.failures(), [])
        self.write('api/lsd_core/index.html', '<a href="#absent%3Cu8%3E">Missing</a>')
        self.assertTrue(any('missing anchor' in failure for failure in self.failures()))

    def test_source_ranges_require_every_line_and_forward_order(self):
        self.write('api/lsd_core/index.html', '<a href="../src/lsd_core/code.rs.html#1-3">Source</a>')
        self.write('api/src/lsd_core/code.rs.html', '<a id="1"></a><a id="2"></a><a id="3"></a>')
        self.assertEqual(self.failures(), [])
        self.write('api/src/lsd_core/code.rs.html', '<a id="1"></a><a id="3"></a>')
        self.assertTrue(self.failures())
        self.write('api/lsd_core/index.html', '<a href="../src/lsd_core/code.rs.html#3-1">Source</a>')
        self.assertTrue(self.failures())

    def test_range_syntax_does_not_exempt_ordinary_pages(self):
        self.write('api/lsd_core/index.html', '<a href="ordinary.html#1-2">Invalid</a>')
        self.write('api/lsd_core/ordinary.html', '<a id="1"></a><a id="2"></a>')
        self.assertTrue(self.failures())

    def test_reexport_redirect_checks_canonical_fragment(self):
        self.write('api/lsd_core/index.html', '<a href="alias.html#method.tick">Method</a>')
        self.write('api/lsd_core/alias.html',
                   '<meta http-equiv="refresh" content="0;URL=canonical.html">')
        self.write('api/lsd_core/canonical.html', '<section id="method.tick"></section>')
        self.assertEqual(self.failures(), [])
        self.write('api/lsd_core/canonical.html', '<section id="method.other"></section>')
        self.assertTrue(self.failures())

    def test_redirect_cycles_and_paths_outside_site_fail(self):
        self.write('api/lsd_core/index.html', '<a href="a.html#method.tick">Method</a>')
        self.write('api/lsd_core/a.html', '<meta http-equiv="refresh" content="0;URL=b.html">')
        self.write('api/lsd_core/b.html', '<meta http-equiv="refresh" content="0;URL=a.html">')
        self.assertTrue(self.failures())
        self.write('api/lsd_core/b.html', '<meta http-equiv="refresh" content="0;URL=../../../outside.html">')
        self.assertTrue(any('leaves site' in failure for failure in self.failures()))

    def rustdoc(self):
        source = self.root / 'rustdoc'
        for crate in stage_api.CRATES:
            path = source / crate / 'index.html'
            path.parent.mkdir(parents=True)
            path.write_text('<p>Owned API fixture</p>', encoding='utf-8')
        (source / 'static.files').mkdir()
        (source / 'search.index').mkdir()
        (source / '.lock').write_text('local lock', encoding='utf-8')
        self.write('API_REFERENCE/index.html', '<p>API guide</p>')
        return source

    def test_staging_records_revision_and_preserves_existing_registries(self):
        source = self.rustdoc()
        trait = source / 'lsd_core' / 'trait.Host.html'
        trait.write_text('<script src="../trait.impl/lsd_core/trait.Host.js"></script>'
                         '<script src="../trait.impl/lsd_core/trait.Existing.js"></script>', encoding='utf-8')
        existing = source / 'trait.impl' / 'lsd_core' / 'trait.Existing.js'
        existing.parent.mkdir(parents=True)
        existing.write_text('/* existing implementors */', encoding='utf-8')
        manifest = stage_api.stage(source, self.site, 'a' * 40)
        self.assertEqual(manifest['revision'], 'a' * 40)
        self.assertEqual(manifest['empty_trait_registries'], ['trait.impl/lsd_core/trait.Host.js'])
        self.assertEqual((self.site / 'api/trait.impl/lsd_core/trait.Existing.js').read_text(),
                         '/* existing implementors */')
        self.assertFalse((self.site / 'api/.lock').exists())
        self.assertEqual(self.failures(require_api=True), [])
        manifest['features'] = 'desktop'
        self.write('api/build.json', json.dumps(manifest))
        self.assertTrue(self.failures(require_api=True))

    def test_staging_rejects_existing_destination_without_changing_it(self):
        source = self.rustdoc()
        marker = self.write('api/keep.txt', 'retained')
        with self.assertRaisesRegex(ValueError, 'already exists'):
            stage_api.stage(source, self.site, 'a' * 40)
        self.assertEqual(marker.read_text(), 'retained')

    def test_incomplete_input_and_invalid_revision_do_not_stage(self):
        source = self.rustdoc()
        with self.assertRaisesRegex(ValueError, 'full Git commit'):
            stage_api.stage(source, self.site, 'main')
        (source / 'lsd_assets/index.html').unlink()
        with self.assertRaisesRegex(ValueError, 'lsd_assets'):
            stage_api.stage(source, self.site, 'a' * 40)
        self.assertFalse((self.site / 'api').exists())
        self.assertTrue(self.failures(require_api=True))


if __name__ == '__main__':
    unittest.main()
