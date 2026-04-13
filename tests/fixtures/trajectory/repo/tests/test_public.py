import unittest

from cachelib import LRUCache


class PublicTests(unittest.TestCase):
    def test_items_promote_recency(self):
        cache = LRUCache(3)
        cache["a"] = 1
        cache["b"] = 2
        cache["c"] = 3
        self.assertEqual(cache.keys(), ["c", "b", "a"])
        self.assertEqual(cache["a"], 1)
        self.assertEqual(cache.keys(), ["a", "c", "b"])
