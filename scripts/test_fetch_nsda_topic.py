"""Offline tests for fetch_nsda_topic: heading targeting + period choice."""

import subprocess
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from fetch_nsda_topic import extract_pf_topic  # noqa: E402

# Mirrors the real Divi page: duplicate id="pf" blocks, an LD section
# first, the PF resolution as a sibling text module after the <h3>, and
# the next PF period pre-published below the current one.
FIXTURE = """
<div id="pf"><h3>Lincoln-Douglas Debate \u2013 2026 September/October Topic</h3></div>
<div><strong>Resolved: An LD resolution that must never be picked.</strong></div>
<div id="pf"><h3><strong>Public Forum Debate \u2013 2026 September/October Topic</strong></h3></div>
<div><strong>Resolved: The United States federal government should enact a
moratorium on hyperscale data center construction.</strong></p>
<p><em>A total of 530 coaches and 2,401 students voted for the resolution.</em></div>
<div id="pf"><h3>Public Forum Debate \u2013 2026 November/December Topic</h3></div>
<div><strong>Resolved: European members of NATO should ratify a treaty.</strong></div>
<h3>Public Forum Debate 2026-2027 Potential Topics</h3>
<p>Resolved: A potential topic that must never be picked.</p>
"""

SEPT_RES = (
    "Resolved: The United States federal government should enact a "
    "moratorium on hyperscale data center construction."
)


class ExtractTests(unittest.TestCase):
    def test_october_picks_september_october_not_ld_or_potential(self):
        topic = extract_pf_topic(FIXTURE, date(2026, 10, 19))
        self.assertEqual(topic, {"resolution": SEPT_RES, "period": "2026 September/October"})

    def test_november_picks_the_new_period_without_a_code_change(self):
        topic = extract_pf_topic(FIXTURE, date(2026, 11, 2))
        self.assertEqual(topic["period"], "2026 November/December")
        self.assertTrue(topic["resolution"].startswith("Resolved: European members"))

    def test_trailing_sentence_after_resolution_is_not_swallowed(self):
        # Issue #36 repro: page copy after the resolution's first period.
        html = (
            '<h2>Public Forum Debate \u2013 2026 September/October Topic</h2>'
            '<p>Resolved: The US should do X. Registration for the topic webinar opens Monday.</p>'
            '<h2>Next</h2>'
        )
        topic = extract_pf_topic(html, date(2026, 9, 15))
        self.assertEqual(topic["resolution"], "Resolved: The US should do X.")

    def test_abbreviation_periods_stay_inside_the_resolution(self):
        html = (
            '<h2>Public Forum Debate \u2013 2026 September/October Topic</h2>'
            '<p>Resolved: The U.S. should spend 3.5 million dollars. Registration opens Monday.</p>'
        )
        topic = extract_pf_topic(html, date(2026, 9, 15))
        self.assertEqual(topic["resolution"], "Resolved: The U.S. should spend 3.5 million dollars.")

    def test_missing_pf_heading_is_an_error(self):
        with self.assertRaises(ValueError):
            extract_pf_topic("<h3>Policy Debate Topic</h3>", date(2026, 10, 19))

    def test_failure_keeps_fallback_and_never_writes_empty(self):
        script = Path(__file__).parent / "fetch_nsda_topic.py"
        with tempfile.TemporaryDirectory() as tmp:
            fallback = Path(tmp) / "topics.json"
            fallback.write_text('{"resolution": "Resolved: Bundled fallback.", "period": "2026 September/October"}\n')
            bad_html = Path(tmp) / "bad.html"
            bad_html.write_text("<html>no topics here</html>")
            out = Path(tmp) / "out.json"
            proc = subprocess.run(
                [sys.executable, str(script), "--html-file", str(bad_html), "--output", str(out), "--fallback", str(fallback)],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            self.assertIn("::error::", proc.stderr)
            self.assertIn("Bundled fallback", out.read_text())


if __name__ == "__main__":
    unittest.main()
