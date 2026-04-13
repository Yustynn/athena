Template cache should allow `capacity=0` without crashing.

Update cache implementation so writes are discarded when capacity is zero.
Keep existing recency behavior unchanged for positive capacities.
