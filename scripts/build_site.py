#!/usr/bin/env python3
"""Build the static project site for Halftone into a single output directory.

The site has three parts:

- ``index.html``: the repository README rendered from Markdown.
- ``downloads.html``: assets of the latest GitHub release, fetched at build time.
- ``docs/``: mdBook documentation built from ``docs/``.

Run from anywhere::

    python3 scripts/build_site.py

The latest release is fetched from the GitHub API. Set ``GITHUB_TOKEN`` (or
``GH_TOKEN``) to raise the API rate limit. A repository with no releases yet is
handled gracefully; any other API failure aborts the build on purpose, so the
downloads section never silently points at stale or invented binaries.
"""

from __future__ import annotations

import argparse
import html
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

try:
    import markdown
except ImportError:  # pragma: no cover - exercised only without the dependency
    sys.exit(
        "Missing dependency 'Markdown'. "
        "Install with: pip install -r website/requirements.txt"
    )

REPO_ROOT = Path(__file__).resolve().parents[1]

# Target triples to present, with the label and architecture shown to users.
PLATFORMS = (
    ("x86_64-unknown-linux-gnu", "Linux", "x86_64"),
    ("aarch64-unknown-linux-gnu", "Linux", "arm64"),
    ("x86_64-apple-darwin", "macOS", "Intel"),
    ("aarch64-apple-darwin", "macOS", "Apple Silicon"),
    ("x86_64-pc-windows-msvc", "Windows", "x86_64"),
)

# Applications packaged by the release workflow.
COMPONENTS = (
    ("halftone-cli", "Command line"),
    ("halftone-gui", "Desktop app"),
)

# Archive extensions produced by cargo-dist, in display order.
ARTIFACT_KINDS = (
    ("tar.xz", "tar.xz"),
    ("zip", "zip"),
    ("msi", "installer (.msi)"),
)

DESCRIPTION = (
    "Halftone converts images and 3D OBJ meshes into vector halftone SVG. "
    "Prebuilt binaries for Linux, macOS and Windows."
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Halftone project site.")
    parser.add_argument("--repo", default="apanoush/halftone", help="GitHub owner/repo")
    parser.add_argument(
        "--output", default="site", help="output directory, relative to the repo root"
    )
    return parser.parse_args()


def fetch_latest_release(repo: str) -> dict | None:
    """Return the latest release, or ``None`` when the repository has none."""
    url = f"https://api.github.com/repos/{repo}/releases/latest"
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "halftone-site-builder",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    request = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise SystemExit(f"GitHub API error {error.code}: {error.reason}") from error
    except urllib.error.URLError as error:
        raise SystemExit(f"Could not reach the GitHub API: {error.reason}") from error


def human_size(size: int) -> str:
    value = float(size)
    for unit in ("B", "KB", "MB", "GB"):
        if value < 1024 or unit == "GB":
            return f"{value:.0f} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} GB"


def rewrite_doc_links(text: str) -> str:
    """Point Markdown links into ``docs/`` at the built HTML."""
    pattern = re.compile(r"\]\(doc(?:s)?/([^)\s]+?)\.md(#[^)\s]*)?")

    def replace(match: re.Match[str]) -> str:
        return f"](docs/{match.group(1)}.html{match.group(2) or ''})"

    return pattern.sub(replace, text)


def render_markdown(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    text = rewrite_doc_links(text)
    return markdown.markdown(text, extensions=["extra", "toc", "sane_lists"])


def index_assets(release: dict) -> dict[str, dict]:
    return {asset["name"]: asset for asset in release.get("assets", [])}


def render_cell(assets: dict[str, dict], component: str, triple: str) -> str:
    base = f"{component}-{triple}"
    items: list[str] = []
    checksum: dict | None = None
    for extension, label in ARTIFACT_KINDS:
        asset = assets.get(f"{base}.{extension}")
        if asset:
            url = html.escape(asset["browser_download_url"], quote=True)
            items.append(
                f'<a href="{url}">{label}</a> '
                f'<span class="muted">{human_size(asset["size"])}</span>'
            )
        if checksum is None:
            checksum = assets.get(f"{base}.{extension}.sha256")
    if checksum:
        url = html.escape(checksum["browser_download_url"], quote=True)
        items.append(f'<a class="muted" href="{url}">sha256</a>')
    if not items:
        return '<span class="muted">not built</span>'
    return (
        '<ul class="files">' + "".join(f"<li>{item}</li>" for item in items) + "</ul>"
    )


def render_downloads_table(release: dict) -> str:
    assets = index_assets(release)
    head = "".join(f'<th scope="col">{label}</th>' for _, label in COMPONENTS)
    rows = []
    for triple, platform, arch in PLATFORMS:
        cells = "".join(
            f"<td>{render_cell(assets, component, triple)}</td>"
            for component, _ in COMPONENTS
        )
        rows.append(
            f'<tr><th scope="row">{platform}<br>'
            f'<span class="muted">{arch}</span></th>{cells}</tr>'
        )
    return (
        '<div class="table-scroll"><table class="downloads">'
        f'<thead><tr><th scope="col">Platform</th>{head}</tr></thead>'
        f"<tbody>{''.join(rows)}</tbody></table></div>"
    )


def downloads_page(release: dict | None) -> str:
    if release is None:
        return (
            "<h1>Downloads</h1>"
            "<p>No releases have been published yet. Releases are produced by the "
            "<code>release</code> workflow when a version tag is pushed.</p>"
        )
    tag = html.escape(release["tag_name"])
    page_url = html.escape(release["html_url"], quote=True)
    published = (release.get("published_at") or "")[:10]
    when = f' <span class="muted">({published})</span>' if published else ""
    return "\n".join(
        [
            "<h1>Downloads</h1>",
            (
                f"<p>Latest release <strong>{tag}</strong>{when}. "
                f'<a href="{page_url}">Release notes and checksums</a>.</p>'
            ),
            render_downloads_table(release),
            (
                '<p class="muted">On Windows, install the <code>.msi</code>. On Linux and '
                "macOS, unpack the <code>tar.xz</code> archive. Verify a download with its "
                "matching <code>.sha256</code> file.</p>"
            ),
        ]
    )


def downloads_highlight(release: dict | None) -> str:
    if release is None:
        return (
            '<section class="callout"><h2>Downloads</h2>'
            "<p>No releases have been published yet.</p></section>"
        )
    tag = html.escape(release["tag_name"])
    page_url = html.escape(release["html_url"], quote=True)
    return (
        '<section class="callout"><h2>Downloads</h2>'
        f"<p>Latest release <strong>{tag}</strong>: prebuilt binaries for Linux, macOS "
        "and Windows, plus an MSI installer.</p>"
        '<p><a class="button" href="downloads.html">All downloads</a> '
        f'<a class="button secondary" href="{page_url}">Release notes</a></p></section>'
    )


def insert_downloads_highlight(readme_html: str, release: dict | None) -> str:
    """Place the downloads callout after the README intro, before the first section."""
    highlight = downloads_highlight(release)
    marker = readme_html.find("<h2")
    if marker == -1:
        return readme_html + "\n" + highlight
    return readme_html[:marker] + highlight + "\n" + readme_html[marker:]


def render_page(template: str, *, title: str, content: str, active: str) -> str:
    def current(name: str) -> str:
        return ' aria-current="page"' if name == active else ""

    replacements = {
        "{{TITLE}}": title,
        "{{DESCRIPTION}}": DESCRIPTION,
        "{{ROOT}}": "",
        "{{HOME_CURRENT}}": current("home"),
        "{{DOWNLOADS_CURRENT}}": current("downloads"),
        "{{DOCS_CURRENT}}": current("docs"),
        "{{CONTENT}}": content,
    }
    for token, value in replacements.items():
        template = template.replace(token, value)
    return template


def main() -> None:
    args = parse_args()
    output = (REPO_ROOT / args.output).resolve()
    if REPO_ROOT != output and REPO_ROOT not in output.parents:
        raise SystemExit(f"Refusing to write outside the repository: {output}")
    if shutil.which("mdbook") is None:
        raise SystemExit("mdbook is required. Install with: cargo install mdbook")

    template = (REPO_ROOT / "website" / "template.html").read_text(encoding="utf-8")

    release = fetch_latest_release(args.repo)

    if output.exists():
        shutil.rmtree(output)
    (output / "assets").mkdir(parents=True)

    subprocess.run(["mdbook", "build"], cwd=REPO_ROOT, check=True)

    home_content = insert_downloads_highlight(
        render_markdown(REPO_ROOT / "README.md"), release
    )
    (output / "index.html").write_text(
        render_page(
            template,
            title="Halftone - vector halftone SVG",
            content=home_content,
            active="home",
        ),
        encoding="utf-8",
    )

    (output / "downloads.html").write_text(
        render_page(
            template,
            title="Downloads - Halftone",
            content=downloads_page(release),
            active="downloads",
        ),
        encoding="utf-8",
    )

    shutil.copy2(REPO_ROOT / "website" / "style.css", output / "assets" / "style.css")

    release_label = release["tag_name"] if release else "no releases yet"
    print(f"Built site in {output} (latest release: {release_label})")


if __name__ == "__main__":
    main()
