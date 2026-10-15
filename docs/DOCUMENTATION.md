# Documentation workflow

The [project handbook](https://baileypillon.github.io/lsd-dream-emulator-bevy/)
publishes the Markdown in `docs/`. The same files remain readable on GitHub.
Search, navigation, theme selection, and code copying work without an account.
The site uses system fonts and locally built search assets; no analytics or
external font service is configured.

## Preview a change

Use Python 3.10 or newer. A virtual environment keeps documentation dependencies
separate from the game and your other Python tools:

```sh
python -m venv .private/docs-venv
```

Activate it with `.private/docs-venv/Scripts/Activate.ps1` in PowerShell or
`source .private/docs-venv/bin/activate` on Linux/macOS, then run:

```sh
python -m pip install -r tools/requirements-docs.txt
python -m mkdocs serve
```

Open the local URL printed by MkDocs. The server includes the project's URL
prefix, matching its GitHub Pages deployment. Stop it with Ctrl+C.

For the checks used in CI:

```sh
python tools/check_docs.py
python -m mkdocs build --strict
python tools/check_site.py .private/site
```

The first check validates repository links, heading anchors, JSON, and SVGs.
The strict build checks navigation and Markdown links. The final check walks
the generated HTML, its links and fragments, images, scripts, and stylesheets.

## Edit and organize content

- Update the existing Markdown file; avoid a separate copy for the website.
- Add new pages to the appropriate section of `mkdocs.yml` and the
  [documentation index](README.md). New pages must be discoverable in both.
- Use repository-relative links. MkDocs converts links between documentation
  pages; `tools/docs_site.py` directs source-code and root-policy links to
  their GitHub locations without altering the Markdown files.
- Keep the current status summary distinct from historical recovery notes.
  Preserve frozen measurements and publish only bounded evidence.
- Keep generated output, local environments, game files, and private analysis
  outside version control. The configured output is `.private/site`.

## Publishing and ownership

Pull requests build and validate the site in the required **Repository quality**
check. After a relevant change merges to protected `main`, the
[Documentation workflow](../.github/workflows/docs.yml) builds the site again
and publishes its artifact through GitHub Pages. A failed build cannot deploy.
Manual dispatch can republish the current `main` revision.

Deployment permissions are confined to the deployment job. Pull request checks
have read-only repository access and never deploy. Actions and Python packages
are pinned; Dependabot proposes updates for review.

The site uses [MkDocs](https://www.mkdocs.org/) and
[Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).
The original doorway artwork and palette are shared with the README. See
[attribution](../NOTICE.md) for dependency and original-content boundaries.
