"""Stage a fresh default-feature Rustdoc build inside the project handbook."""

import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import shutil
import sys
from urllib.parse import unquote, urlsplit

CRATES = ('lsd_core', 'lsd_assets', 'lsd_bevy')


class TraitScripts(HTMLParser):
    def __init__(self):
        super().__init__()
        self.sources = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'script' and attrs.get('src'):
            self.sources.append(attrs['src'])


def supply_empty_trait_registries(destination):
    """Rustdoc omits registries when no cross-crate implementors were emitted."""
    supplied = set()
    for crate in CRATES:
        for page in (destination / crate).rglob('trait.*.html'):
            parser = TraitScripts()
            parser.feed(page.read_text(encoding='utf-8'))
            for source in parser.sources:
                parsed = urlsplit(source)
                if parsed.scheme or parsed.netloc:
                    continue
                target = (page.parent / unquote(parsed.path)).resolve()
                if (target.is_relative_to(destination / 'trait.impl')
                        and target.name.startswith('trait.') and target.suffix == '.js'
                        and not target.exists()):
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_text(
                        'if (window.register_implementors) { window.register_implementors({}); }'
                        ' else { window.pending_implementors = {}; }\n', encoding='utf-8')
                    supplied.add(target.relative_to(destination).as_posix())
    return sorted(supplied)


def stage(rustdoc, site, revision):
    rustdoc = Path(rustdoc).resolve()
    site = Path(site).resolve()
    destination = site / 'api'
    if not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise ValueError('Revision must be the full Git commit SHA.')
    if not (site / 'index.html').is_file():
        raise ValueError('Build the handbook before staging API documentation.')
    if not (site / 'API_REFERENCE' / 'index.html').is_file():
        raise ValueError('The handbook must contain the API reference guide.')
    if destination.exists():
        raise ValueError('API destination already exists; rebuild the handbook first.')
    for crate in CRATES:
        if not (rustdoc / crate / 'index.html').is_file():
            raise ValueError(f'Missing generated API documentation for {crate}.')
    if not (rustdoc / 'static.files').is_dir() or not (rustdoc / 'search.index').is_dir():
        raise ValueError('Expected Rustdoc static assets and search index.')
    if rustdoc == site or rustdoc.is_relative_to(site) or site.is_relative_to(rustdoc):
        raise ValueError('Rustdoc input and handbook output must be separate directories.')

    shutil.copytree(rustdoc, destination, ignore=shutil.ignore_patterns('.lock'))
    # Cargo emits help/settings links to a root index but does not create it.
    # Send readers back to the handbook's crate directory and scope explanation.
    (destination / 'index.html').write_text(
        '<!doctype html><html lang="en"><meta charset="utf-8">'
        '<meta http-equiv="refresh" content="0; url=../API_REFERENCE/">'
        '<title>LSD: Dream Emulator — API reference</title>'
        '<p><a href="../API_REFERENCE/">Open the API reference guide</a>.</p>'
        '</html>\n', encoding='utf-8')
    manifest = {
        'revision': revision,
        'source': 'https://github.com/BaileyPillon/lsd-dream-emulator-bevy/tree/' + revision,
        'features': 'default',
        'command': 'cargo doc --workspace --lib --no-deps --locked',
        'crates': {crate: f'{crate}/index.html' for crate in CRATES},
        'empty_trait_registries': supply_empty_trait_registries(destination),
    }
    (destination / 'build.json').write_text(
        json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('rustdoc', type=Path, help='Fresh Cargo documentation output directory')
    parser.add_argument('site', type=Path, help='Built MkDocs site directory')
    parser.add_argument('--revision', required=True, help='Full source commit SHA')
    args = parser.parse_args(argv)
    try:
        manifest = stage(args.rustdoc, args.site, args.revision)
    except (OSError, ValueError) as error:
        print(f'API staging failed: {error}', file=sys.stderr)
        return 1
    print(json.dumps({'status': 'staged', **manifest}, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
