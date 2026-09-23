import unittest
from audit_retirement import analyze


class RetirementLogTests(unittest.TestCase):
    def test_delayed_cleanup_remains_failure_after_eventual_recovery(self):
        cycles = analyze("""t=68.247 Active { since: 57s } -> Restoring { since: 68s }
summon retirement: references=None task_proxies_clear=true
t=71.252 failure=restore_timeout; cleanup still pending
summon removed: outcome=destroyed retirement=frame-end-drained
t=105.362 Fault -> Normal""")
        self.assertEqual(cycles[0]["status"], "FAIL")
        self.assertEqual(cycles[0]["seconds"], 37.115)

    def test_body_return_without_destructor_is_not_success(self):
        self.assertEqual(analyze("t=1 Active -> Restoring\nt=2 Restoring -> Normal")[0]["status"], "FAIL")
        self.assertEqual(analyze("t=1 Active -> Restoring")[0]["status"], "INCONCLUSIVE")
        self.assertEqual(analyze("t=1 Active -> Restoring\noutcome=destroyed\nt=2 Restoring -> Normal")[0]["status"], "PASS")


if __name__ == "__main__":
    unittest.main()
