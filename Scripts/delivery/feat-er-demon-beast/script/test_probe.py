"""Synthetic evidence tests; never presented as game evidence."""
import copy
import unittest
from probe import analyze


def failure_trace():
    body = dict(handle="player", address=100, render=True, alpha=1, tint_alpha=1,
                debug_flags=0, disable_hit=False, override=0, hp=100, position=[0, 0, 0])
    state = dict(body=body, beast=None, camera=0, manip_owner=0, patches=0,
                 linked=False, pending_spawn=False, owned_count=0)
    rows = [dict(kind="command", t=0, case=6, accepted=True, action="start")]
    for n in range(11):
        rows.append(dict(kind="frame", t=n/20, case=6, dropped=0,
                         before="normal", phase="normal", enter=n == 2, revert=False,
                         pre=copy.deepcopy(state), post=copy.deepcopy(state),
                         error="missing configured NpcParam/NpcThinkParam" if n >= 2 else None))
    rows.append(dict(kind="command", t=.51, case=6, accepted=True, action="end"))
    return rows


class EvidenceAnalysis(unittest.TestCase):
    def test_empty_capture_never_passes(self):
        self.assertTrue(all(r["status"] == "NOT_RUN" for r in analyze([])))

    def test_failed_entry_is_only_partial_without_human_control_check(self):
        result = analyze(failure_trace())[5]
        self.assertEqual(result["status"], "AUTOMATIC_PARTIAL")
        self.assertIn("随后本体", result["human_pending"])

    def test_truncated_capture_is_inconclusive(self):
        self.assertEqual(analyze(failure_trace()[:-1])[5]["status"], "INCONCLUSIVE")

    def test_dropped_frames_or_large_gap_cannot_prove_deadline(self):
        for field, value in [("dropped", 1), ("t", 6)]:
            with self.subTest(field=field):
                rows = failure_trace()
                rows[5][field] = value
                self.assertEqual(analyze(rows)[5]["status"], "INCONCLUSIVE")

    def test_unrestored_body_fails_even_when_state_says_normal(self):
        rows = failure_trace()
        rows[3]["post"]["body"]["render"] = False
        self.assertEqual(analyze(rows)[5]["status"], "FAIL")

    def test_repeat_case_is_not_merged_into_false_positive(self):
        rows = failure_trace()
        self.assertEqual(analyze(rows + rows)[5]["status"], "INCONCLUSIVE")


if __name__ == "__main__":
    unittest.main()
