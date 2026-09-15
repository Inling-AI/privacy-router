# privacy-filter-cache

An in-memory prediction cache for [`privacy-filter`](../privacy-filter). Useful when successive requests repeat earlier messages or tool outputs: unchanged fragments reuse entity predictions, while new fragments run through the model.

## Usage

```rust,no_run
use privacy_filter::{BatchLimits, PrivacyFilter};
use privacy_filter_cache::{CacheLimits, CachedPrivacyFilter};

let filter = PrivacyFilter::from_dir("models/privacy-filter")?;
let mut cache = CachedPrivacyFilter::new(filter, CacheLimits::default())?;
cache.classify_batch(&["Contact alex@example.com"], BatchLimits::default())?;

let result = cache.classify_batch(
    &["Contact alex@example.com", "My name is Alex Example"],
    BatchLimits::default(),
)?;
assert_eq!(result.usage.cached_fragments, 1);
assert_eq!(result.usage.inferred_fragments, 1);
# Ok::<(), privacy_filter::Error>(())
```

The cache owns the model and its decoding options. Enable `gpu` to wrap a GPU-backed filter. Access requires `&mut self`; applications sharing an instance across concurrent requests must coordinate access.

## Reuse and Eviction

Complete fragments are hashed with BLAKE3 before tokenization. Duplicate inputs within one call are inferred once. Results retain input order and share `Arc<[Entity]>` storage. `CacheUsage` separates prior cache hits, within-call duplicates, and newly inferred fragments.

Defaults are 4096 entries and 16 MiB of retained entry data. LRU eviction enforces both limits. An entry larger than the byte budget is returned without caching. The budget excludes model memory, allocator overhead, and results retained by callers.

Failed inference inserts no new predictions. Empty strings consume no model work. `clear_cache` releases entries owned by the cache; callers' existing results remain valid.

## Operational Limits

Changing any part of a fragment requires fresh inference because the model uses bidirectional attention. Each lookup still hashes the full text. Cached predictions contain entity text: choose cache lifetime and tenant boundaries accordingly. Hashed keys do not make retained entity values anonymous, and eviction does not securely erase memory.

Policy decisions belong to the caller. Re-evaluating current rules against cached entities allows rule changes to take effect immediately.

## Benchmark

From the workspace root, with weights downloaded, supply a JSON array of objects containing a `text` field:

```sh
cargo run --release --features gpu -p privacy-filter-cache --example benchmark -- /tmp/fragments.json --full --warm --gpu
```

The benchmark reports cold-prefix, append, and replay work locally. `cargo test -p privacy-filter-cache` covers reuse, deduplication, eviction, and failed-batch behavior.
