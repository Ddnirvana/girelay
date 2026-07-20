import unittest

from task_id import normalize_task_id


class TaskIdTests(unittest.TestCase):
    def test_lowercases_and_collapses_separators(self):
        self.assertEqual(normalize_task_id(" Auth__ Refresh "), "auth-refresh")

    def test_rejects_empty_result(self):
        with self.assertRaises(ValueError):
            normalize_task_id("___")


if __name__ == "__main__":
    unittest.main()
