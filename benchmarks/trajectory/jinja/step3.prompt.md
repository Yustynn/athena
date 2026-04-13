Follow up on same `jinja2.utils.LRUCache` implementation.

Add `pop(key, default=missing)` that removes cached value and returns it.
If key is missing, raise `KeyError` unless caller provided default.
Removing one key must not disturb recency order of remaining keys.
