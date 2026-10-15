"""Validate local links and assets in a built MkDocs site without network access."""

import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import sys
from urllib.parse import unquote, urlsplit


class Document(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.anchors = set()
        self.links = []
        self.feed(path.read_text(encoding='utf-8'))

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get('id'):
            self.anchors.add(attrs['id'])
        if tag == 'a' and attrs.get('name'):
            self.anchors.add(attrs['name'])
        for attribute in ('href', 'src', 'poster'):
            if attrs.get(attribute):
                self.links.append(attrs[attribute])


def check(site, base_path='/lsd-dream-emulator-bevy/'):
    site = Path(site).resolve()
    if not (site / 'index.html').is_file():
        raise ValueError('Expected a built site containing index.html.')
    documents = {path: Document(path) for path in site.rglob('*.html')}
    failures = []
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
                  and unquote(parsed.fragment) not in documents[target].anchors):
                failures.append(f'{source.relative_to(site)}: missing anchor: {destination}')
    return {'html_pages': len(documents), 'local_links_and_assets': count}, failures


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('site', type=Path, help='Generated site directory')
    args = parser.parse_args(argv)
    try:
        counts, failures = check(args.site)
    except (OSError, UnicodeError, ValueError) as error:
        print(f'Site check failed: {error}', file=sys.stderr)
        return 1
    for failure in failures:
        print(failure, file=sys.stderr)
    print(json.dumps({'status': 'failed' if failures else 'passed', **counts}, sort_keys=True))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
