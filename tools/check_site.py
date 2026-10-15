"""Validate local links and assets in a built MkDocs site without network access."""

import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit


class Document(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.anchors = set()
        self.links = []
        self.redirect = None
        self.feed(path.read_text(encoding='utf-8'))

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get('id'):
            self.anchors.add(attrs['id'])
        if tag == 'a' and attrs.get('name'):
            self.anchors.add(attrs['name'])
        if tag == 'meta' and attrs.get('http-equiv', '').lower() == 'refresh':
            refresh = re.fullmatch(r'\s*0\s*;\s*url=(.+)', attrs.get('content', ''), re.I)
            if refresh:
                self.redirect = refresh.group(1).strip()
                self.links.append(self.redirect)
        for attribute in ('href', 'src', 'poster'):
            if attrs.get(attribute):
                self.links.append(attrs[attribute])


def has_anchor(document, fragment, target, site, documents, visited=frozenset()):
    decoded = unquote(fragment)
    if fragment in document.anchors or decoded in document.anchors:
        return True
    # Rustdoc source pages select a line range in JavaScript. Validate every
    # requested line instead of exempting generated source links from checks.
    if target.is_relative_to(site / 'api' / 'src') and re.fullmatch(r'[1-9]\d*-[1-9]\d*', decoded):
        first, last = map(int, decoded.split('-'))
        return (first <= last and last - first < len(document.anchors)
                and all(str(line) in document.anchors for line in range(first, last + 1)))
    # Rustdoc redirects re-export paths to their canonical type pages and
    # preserves the requested fragment. Check the final page, including cycles.
    if document.redirect and target.is_relative_to(site / 'api') and target not in visited:
        redirect = urlsplit(document.redirect)
        if not redirect.scheme and not redirect.netloc and not redirect.path.startswith('/'):
            canonical = (target.parent / unquote(redirect.path)).resolve()
            if canonical.is_relative_to(site) and canonical in documents:
                return has_anchor(documents[canonical], fragment, canonical, site,
                                  documents, visited | {target})
    return False


def check(site, base_path='/lsd-dream-emulator-bevy/', require_api=False):
    site = Path(site).resolve()
    if not (site / 'index.html').is_file():
        raise ValueError('Expected a built site containing index.html.')
    documents = {path: Document(path) for path in site.rglob('*.html')}
    failures = []
    if require_api:
        manifest_path = site / 'api' / 'build.json'
        if not manifest_path.is_file():
            failures.append('Missing API build identity: api/build.json')
        else:
            manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
            if (not isinstance(manifest, dict)
                    or not re.fullmatch(r'[0-9a-f]{40}', str(manifest.get('revision', '')))
                    or manifest.get('features') != 'default'
                    or manifest.get('crates') != {
                        crate: f'{crate}/index.html'
                        for crate in ('lsd_core', 'lsd_assets', 'lsd_bevy')
                    }):
                failures.append('Invalid API build identity or feature scope.')
        for crate in ('lsd_core', 'lsd_assets', 'lsd_bevy'):
            if not (site / 'api' / crate / 'index.html').is_file():
                failures.append(f'Missing API entry point: {crate}')
    count = 0
    for source, document in documents.items():
        for destination in document.links:
            parsed = urlsplit(destination)
            if parsed.scheme or parsed.netloc:
                continue
            count += 1
            path = unquote(parsed.path)
            if path.startswith('/'):
                if not path.startswith(base_path):
                    failures.append(f'{source.relative_to(site)}: wrong site prefix: {destination}')
                    continue
                target = (site / path[len(base_path):]).resolve()
            else:
                target = (source.parent / path).resolve() if path else source
            if not target.is_relative_to(site):
                failures.append(f'{source.relative_to(site)}: link leaves site: {destination}')
                continue
            if target.is_dir():
                target /= 'index.html'
            if not target.is_file():
                failures.append(f'{source.relative_to(site)}: missing target: {destination}')
            elif (parsed.fragment and target in documents
                  and not has_anchor(documents[target], parsed.fragment, target, site, documents)):
                failures.append(f'{source.relative_to(site)}: missing anchor: {destination}')
    return {'html_pages': len(documents), 'local_links_and_assets': count}, failures


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('site', type=Path, help='Generated site directory')
    parser.add_argument('--require-api', action='store_true',
                        help='Also require the three generated libraries and their build identity')
    args = parser.parse_args(argv)
    try:
        counts, failures = check(args.site, require_api=args.require_api)
    except (OSError, UnicodeError, ValueError) as error:
        print(f'Site check failed: {error}', file=sys.stderr)
        return 1
    for failure in failures:
        print(failure, file=sys.stderr)
    print(json.dumps({'status': 'failed' if failures else 'passed', **counts}, sort_keys=True))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
