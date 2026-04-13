Follow up on same cache implementation.

Add `peek(key, default=None)` that returns cached value without changing recency order.
If key is missing, return provided default.
