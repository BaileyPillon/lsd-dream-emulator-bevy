"""Check repository-local Markdown links, JSON evidence, and SVG assets."""

import html
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r'!?\[[^\]\n]*\]\(\s*(?:<([^>\n]+)>|([^\s)]+))(?:\s+"[^"]*")?\s*\)')
DEFINITION = re.compile(r'^\s{0,3}\[[^\]\n]+\]:\s+<?([^\s>]+)>?', re.M)


def without_fences(text):
    lines = []
    fence = None
    for line in text.splitlines():
        marker = re.match(r'^\s{0,3}(`{3,}|~{3,})', line)
        if marker:
            token = marker.group(1)
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
            lines.append('')
        else:
            lines.append(line if fence is None else '')
    return '\n'.join(lines)


def heading_ids(text):
    text = without_fences(text)
    identifiers = set(re.findall(r'\b(?:id|name)=["\']([^"\']+)["\']', text))
    occurrences = {}
    for line in text.splitlines():
        heading = re.match(r'^ {0,3}#{1,6}\s+(.+?)\s*#*$', line)
        if not heading:
            continue
        label = re.sub(r'\[([^\]]+)\]\([^)]*\)', r'\1', heading.group(1))
        label = html.unescape(re.sub(r'<[^>]+>', '', label)).lower()
        slug = re.sub(r'[^\w\s-]', '', label).replace(' ', '-')
        number = occurrences.get(slug, 0)
        occurrences[slug] = number + 1
        identifiers.add(slug if number == 0 else slug + '-' + str(number))
    return identifiers


def reject_nonfinite(value):
    raise ValueError('Non-finite JSON value: ' + value)


def check(root=ROOT):
    root = Path(root).resolve()
    listed = subprocess.check_output(
        ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=root)
    files = sorted({Path(name.decode('utf-8')) for name in listed.split(b'\0') if name})
    failures = []
    counts = {'markdown_files': 0, 'local_links': 0, 'json_files': 0, 'svg_files': 0}
    anchor_cache = {}
    for relative in files:
        path = root / relative
        if path.suffix.lower() == '.md':
            counts['markdown_files'] += 1
            text = without_fences(path.read_text(encoding='utf-8'))
            destinations = [match.group(1) or match.group(2) for match in LINK.finditer(text)]
            destinations += [match.group(1) for match in DEFINITION.finditer(text)]
            for destination in destinations:
                parsed = urlsplit(html.unescape(destination))
                if parsed.scheme or parsed.netloc:
                    continue
                counts['local_links'] += 1
                target = (path.parent / unquote(parsed.path)).resolve() if parsed.path else path
                if not target.is_relative_to(root):
                    failures.append(f'{relative}: link leaves repository: {destination}')
                    continue
                if not target.exists():
                    failures.append(f'{relative}: missing link target: {destination}')
                    continue
                if parsed.fragment and target.suffix.lower() == '.md':
                    if target not in anchor_cache:
                        anchor_cache[target] = heading_ids(target.read_text(encoding='utf-8'))
                    if unquote(parsed.fragment) not in anchor_cache[target]:
                        failures.append(f'{relative}: missing heading: {destination}')
        elif path.suffix.lower() == '.json':
            counts['json_files'] += 1
            try:
                json.loads(path.read_text(encoding='utf-8'), parse_constant=reject_nonfinite)
            except (ValueError, UnicodeError) as error:
                failures.append(f'{relative}: invalid JSON: {error}')
        elif path.suffix.lower() == '.svg':
            counts['svg_files'] += 1
            try:
                document = ET.parse(path)
                if document.getroot().tag != '{http://www.w3.org/2000/svg}svg':
                    raise ValueError('Expected an SVG document root.')
            except (ET.ParseError, ValueError) as error:
                failures.append(f'{relative}: invalid SVG: {error}')
    return counts, failures


def main():
    try:
        counts, failures = check()
    except (OSError, UnicodeError, subprocess.CalledProcessError) as error:
        print('Documentation check failed: ' + str(error), file=sys.stderr)
        return 1
    for failure in failures:
        print(failure, file=sys.stderr)
    print(json.dumps({'status': 'failed' if failures else 'passed', **counts}, sort_keys=True))
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
