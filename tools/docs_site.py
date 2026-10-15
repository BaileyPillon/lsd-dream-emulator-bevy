"""Keep repository-relative source links usable on the documentation site.

Markdown remains the source of truth on GitHub. Links within docs/ are handled
by MkDocs; links to files elsewhere in the repository open their GitHub source.
Fenced code is left unchanged. No game files or private reports are read.
"""

from pathlib import Path
import re
from urllib.parse import quote, unquote, urlsplit, urlunsplit

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / 'docs'
REPOSITORY = 'https://github.com/BaileyPillon/lsd-dream-emulator-bevy'
INLINE_LINK = re.compile(r'!?\[[^\]\n]*\]\(\s*(?:<([^>\n]+)>|([^\s)]+))')
DEFINITION = re.compile(r'^\s{0,3}\[[^\]\n]+\]:\s+<?([^\s>]+)>?')


def source_url(destination, source):
    parsed = urlsplit(destination)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return destination
    target = (source.parent / unquote(parsed.path)).resolve()
    if target.is_relative_to(DOCS):
        return destination
    if not target.is_relative_to(ROOT) or not target.exists():
        raise ValueError(f'Invalid repository link in {source.name}: {destination}')
    view = 'tree' if target.is_dir() else 'blob'
    path = quote(target.relative_to(ROOT).as_posix(), safe='/')
    url = urlsplit(f'{REPOSITORY}/{view}/main/{path}')
    return urlunsplit((url.scheme, url.netloc, url.path, parsed.query, parsed.fragment))


def on_page_markdown(markdown, *, page, **kwargs):
    source = Path(page.file.abs_src_path)
    result = []
    fence = None
    for line in markdown.splitlines(keepends=True):
        marker = re.match(r'^\s{0,3}(`{3,}|~{3,})', line)
        if marker:
            token = marker.group(1)
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
        elif fence is None:
            def rewrite(match):
                group = 1 if match.group(1) is not None else 2
                start, end = match.span(group)
                return (match.string[match.start():start]
                        + source_url(match.group(group), source)
                        + match.string[end:match.end()])

            line = INLINE_LINK.sub(rewrite, line)
            definition = DEFINITION.match(line)
            if definition:
                start, end = definition.span(1)
                line = line[:start] + source_url(definition.group(1), source) + line[end:]
        result.append(line)
    return ''.join(result)
