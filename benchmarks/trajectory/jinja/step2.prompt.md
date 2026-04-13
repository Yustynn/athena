Follow up on same `jinja2.utils.LRUCache` implementation.

Add `peek(key, default=None)` that returns cached value without changing recency order.
Missing keys should return provided default.
