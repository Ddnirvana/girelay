import unittest

from task_id import normalize_task_id


class ReservedTaskIdTests(unittest.TestCase):
    def test_rejects_reserved_git_names(self):
        for value in ["main", "HEAD", "refs"]:
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    normalize_task_id(value)


if __name__ == "__main__":
    unittest.main()
