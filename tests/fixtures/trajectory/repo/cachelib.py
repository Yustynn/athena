from collections import deque


class LRUCache:
    def __init__(self, capacity):
        self.capacity = capacity
        self._mapping = {}
        self._queue = deque()

    def __getitem__(self, key):
        value = self._mapping[key]
        if self._queue[-1] != key:
            self._queue.remove(key)
            self._queue.append(key)
        return value

    def __setitem__(self, key, value):
        if key in self._mapping:
            self._queue.remove(key)
        elif len(self._mapping) == self.capacity:
            del self._mapping[self._queue.popleft()]

        self._queue.append(key)
        self._mapping[key] = value

    def __contains__(self, key):
        return key in self._mapping

    def __len__(self):
        return len(self._mapping)

    def keys(self):
        keys = list(self._queue)
        keys.reverse()
        return keys
