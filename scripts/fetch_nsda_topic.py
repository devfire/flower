#!/usr/bin/env python3
"""Extract the current Public Forum resolution from the NSDA topics page.

Build-time only (issue #13): a browser fetch of speechanddebate.org is
CORS-blocked, so the deploy workflow runs this script and ships the
result as same-origin ``topics.json`` next to the static WASM app.

The page contains several duplicate ``id="pf"`` Divi blocks, so we never
target that anchor. Instead we target the *heading text*
"Public Forum Debate - <period> Topic" and take the "Resolved: ..."
text that follows it. The page pre-publishes the next period alongside
the current one, so among PF headings we pick the latest period whose
start month is not in the future (or the earliest upcoming one if all
are in the future).

On any fetch/parse failure we keep the last bundled ``topics.json``
(the fallback) and say so loudly; we never write an empty file. With no
valid fallback we exit non-zero.

Stdlib only. Testable offline:
    fetch_nsda_topic.py --html-file page.html --today 2026-10-19
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from datetime import date
from html.parser import HTMLParser
from pathlib import Path

SOURCE_URL = "https://www.speechanddebate.org/topics/"
USER_AGENT = "flower-topic-fetch/1.0 (+https://github.com/devfire/flower)"

MONTHS = {
    "january": 1,
    "february": 2,
    "march": 3,
    "april": 4,
    "may": 5,
    "june": 6,
    "july": 7,
    "august": 8,
    "september": 9,
    "october": 10,
    "november": 11,
    "december": 12,
}

PF_HEADING = re.compile(r"^Public Forum Debate\s*[\u2013\u2014-]\s*(.+?)\s*Topic$")
# First sentence-ending period: a period followed by end-of-block or by
# whitespace + a capital/digit/opener. Abbreviation and decimal periods
# ("U.S. should", "3.5 million") fail that test, so they stay inside.
RESOLVED = re.compile(r"Resolved:\s*(.+?\.)(?=\s+(?:[A-Z0-9\"'\u201c\u2018(\[])|\s*$)")


class _HeadingTextParser(HTMLParser):
    """Collect (heading, following-text) pairs in document order."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.blocks: list[tuple[str, list[str]]] = []
        self._heading: list[str] | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag in {"h1", "h2", "h3", "h4"}:
            self._heading = []

    def handle_endtag(self, tag: str) -> None:
        if tag in {"h1", "h2", "h3", "h4"} and self._heading is not None:
            text = " ".join("".join(self._heading).split())
            self.blocks.append((text, []))
            self._heading = None

    def handle_data(self, data: str) -> None:
        if self._heading is not None:
            self._heading.append(data)
        elif self.blocks:
            self.blocks[-1][1].append(data)


def _period_start(period: str) -> date | None:
    """Start date of a period like '2026 September/October', if datable."""
    year_match = re.search(r"(20\d{2})", period)
    if not year_match:
        return None
    months = [MONTHS[word] for word in re.findall(r"[A-Za-z]+", period.lower()) if word in MONTHS]
    if not months:
        return None
    return date(int(year_match.group(1)), months[0], 1)


def extract_pf_topic(html: str, today: date) -> dict[str, str]:
    """Return {resolution, period} for the PF period current on `today`.

    Raises ValueError when no PF heading + Resolved text is found.
    """
    parser = _HeadingTextParser()
    parser.feed(html)
    candidates: list[tuple[date | None, str, str]] = []
    for heading, following in parser.blocks:
        match = PF_HEADING.match(heading)
        if not match:
            continue
        period = " ".join(match.group(1).split())
        text = " ".join(" ".join(following).split())
        resolved = RESOLVED.search(text)
        if resolved:
            resolution = "Resolved: " + " ".join(resolved.group(1).split())
            candidates.append((_period_start(period), period, resolution))
    if not candidates:
        raise ValueError("no 'Public Forum Debate - ... Topic' heading with a Resolved: text")
    dated = [(start, period, res) for start, period, res in candidates if start is not None]
    if dated:
        current = [c for c in dated if c[0] <= today]
        chosen = max(current, key=lambda c: c[0]) if current else min(dated, key=lambda c: c[0])
    else:
        chosen = candidates[0]
    return {"resolution": chosen[2], "period": chosen[1]}


def _valid_topic(path: Path) -> dict[str, str] | None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if isinstance(data, dict) and str(data.get("resolution", "")).startswith("Resolved:"):
        return {str(k): str(v) for k, v in data.items()}
    return None


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--url", default=SOURCE_URL)
    ap.add_argument("--html-file", type=Path, help="read this saved page instead of fetching")
    ap.add_argument("--output", type=Path, default=Path("topics.json"))
    ap.add_argument("--fallback", type=Path, default=None, help="last bundled file (default: --output)")
    ap.add_argument("--today", type=date.fromisoformat, default=date.today())
    ap.add_argument("--retrieved", default=None, help="retrieved date stamp (default: --today)")
    args = ap.parse_args(argv)
    fallback_path = args.fallback or args.output
    try:
        if args.html_file:
            html = args.html_file.read_text(encoding="utf-8", errors="ignore")
        else:
            request = urllib.request.Request(args.url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=30) as response:  # noqa: S310 - fixed https source
                html = response.read().decode("utf-8", errors="ignore")
        topic = extract_pf_topic(html, args.today)
    except Exception as exc:  # noqa: BLE001 - any failure falls back, loudly
        fallback = _valid_topic(fallback_path)
        if fallback is None:
            print(f"::error::NSDA topic extraction failed ({exc}) and no valid fallback at {fallback_path}", file=sys.stderr)
            return 1
        if fallback_path != args.output:
            args.output.write_text(json.dumps(fallback, indent=2) + "\n", encoding="utf-8")
        print(f"::error::NSDA topic extraction failed ({exc}); keeping last bundled topic from {fallback_path}", file=sys.stderr)
        return 0
    payload = {
        "resolution": topic["resolution"],
        "period": topic["period"],
        "retrieved": args.retrieved or args.today.isoformat(),
        "source": SOURCE_URL,
    }
    args.output.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"NSDA PF topic ({payload['period']}): {payload['resolution']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
