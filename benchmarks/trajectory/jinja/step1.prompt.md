Jinja template cache should allow `LRUCache(0)` without crashing.

Update `jinja2.utils.LRUCache` so writes are discarded when capacity is zero.
Do not change recency semantics for positive capacities.
