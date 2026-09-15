# privacy-filter

Rust inference library for [OpenAI Privacy Filter](https://huggingface.co/openai/privacy-filter), powered by Burn. Detects names, addresses, dates, email addresses, phone numbers, URLs, account numbers, and secrets. Returns scored text spans for applications to review or redact.

## Getting Started

Requires Rust 1.92+. Download the model from the workspace root with `mise run model:download`, or supply a directory containing `config.json`, `tokenizer.json`, and `model.safetensors`.

```rust,no_run
use privacy_filter::PrivacyFilter;

let filter = PrivacyFilter::from_dir("models/privacy-filter")?;
let text = "Contact alex@example.com";
for entity in filter.classify(text)? {
    assert_eq!(&text[entity.start..entity.end], entity.word);
    println!("{}: {:.3}", entity.entity_group, entity.score);
}
# Ok::<(), privacy_filter::Error>(())
```

Keep the filter alive across requests: construction loads the weights and tokenizer. The default backend is CPU/f32. Full-model f32 weights require approximately 5.6 GB, plus working memory.

## Results

`classify` returns `Vec<Entity>`. Each entity has a category, confidence score, half-open UTF-8 byte range (`start..end`), and the exact covered substring (`word`). Covered whitespace is retained. Confidence is the mean selected-label softmax probability across the entity's tokens.

## Filters

Recognition is a `Filter` over one text: a filter declares the category it reports and what its hits mean (`FilterKind`), then returns entities. The model is one filter; deterministic rules are others. Filters never call each other, so adding a recognition method means adding an implementation and nothing else.

`FilterKind` carries the parameters the rest of the system needs:

- `Probabilistic` — hits carry confidence and may be wrong. This is the model.
- `Authoritative` — the shape *is* the category, so probabilistic hits in that category are discarded. Emails are recognized this way: an address either matches the form or is not there at all, and measured on real console traffic most email-labeled model hits were permission strings such as `-rwxr-xr-x@`.
- `Additive` — the pattern covers part of the space and the model keeps contributing the rest. Mainland China mobile numbers work this way: the eleven-digit form is exact, but number formats differ by region, so the model still reports numbers the pattern cannot describe.

`Filters::deterministic()` returns the built-in set: email, mainland China mobile, mainland ID card, bank card, unified social credit code, MAC, and IP. Shape checks live with the filter that owns them — checksums (GB 11643, the mod-31 credit-code digit), Luhn plus issuer prefixes for cards, and for addresses a routability test: loopback, private, link-local, carrier-grade NAT, multicast, and reserved ranges are local-environment coordinates rather than leaked secrets, so they are not hits at all and need no release rule. Deterministic hits score `PatternFilter::SCORE`, and `Filters::combine` merges them with model hits: within one category deterministic evidence wins, and overlapping categories report the more specific one (a MAC-shaped run also appears inside a compressed IPv6 literal). `Filters::kind_of` is the single answer to "how is this category recognized" — rules and the console read it rather than keeping their own list.

`predict_tokens` exposes token offsets and raw logits when an application needs its own decoding. `Options::decoding` selects simple span aggregation or Viterbi decoding with zero transition bias.

## Batching and GPU

`classify_batch` and `predict_tokens_batch` preserve input order. Inputs are packed without padding, with independent attention and position encoding. `BatchLimits` bounds sequences and tokens per microbatch. Empty strings produce empty results.

`Options::max_tokens` defaults to 4096. An oversized input returns `Error::InputTooLong`; callers must split it or choose a supported larger limit. Microbatching does not split individual inputs.

Enable the `gpu` feature to use WGPU; macOS uses Metal:

```rust,no_run
# #[cfg(feature = "gpu")]
# fn example() -> privacy_filter::Result<()> {
use privacy_filter::{BatchLimits, Gpu, GpuDevice, Options, PrivacyFilter};

let filter = PrivacyFilter::<Gpu>::from_dir_on_device(
    "models/privacy-filter", Options::default(), &GpuDevice::default(),
)?;
let results = filter.classify_batch(
    &["Contact alex@example.com", "My name is Alex Example"],
    BatchLimits::default(),
)?;
# Ok(())
# }
```

## Observability

The `*_observed` methods accept an `observation::Observation` and caller-provided `Observer`. Events report tokenization, batch planning, layer submission, input completion, and the final outcome. Metrics include forward inference time and completed-token throughput.

Callbacks run synchronously and should return quickly. Layer submission reports queued work, not device completion. First-result timing measures an internal classification milestone; results are returned after the batch completes. Event payloads omit input text and entity values.

## Validation

`cargo test -p privacy-filter` runs CPU behavior tests and small-model comparisons against Transformers. Full-model GPU checks require downloaded weights. Strict logit parity on real long inputs remains unresolved; matching top labels alone does not establish numerical agreement. False positives and false negatives remain possible. A 128K context and additional GPU platforms have not been validated.

Use [`privacy-filter-cache`](../privacy-filter-cache) to reuse predictions for repeated fragments.
