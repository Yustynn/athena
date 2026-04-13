Follow up on same cache implementation.

Add `pop(key, default=missing)` that removes cached value and returns it.
If key is missing, raise `KeyError` unless caller provided default.
Removing one key must not disturb recency order of remaining keys.
