# Privacy model artifacts

`ModelFile` defines the required config, tokenizer and weights filenames and their
pinned SHA-256 hashes and sizes. The inference library, router cache identity,
verification command and downloader consume this type. The default feature set
has no networking dependencies; enable `download` to prepare artifacts.

## Download lifecycle

1. Verify existing files. A complete valid directory needs no network access.
   A corrupt existing file is reported and preserved.
2. Before downloading any missing artifact, benchmark **all** selected providers
   concurrently on the real weights URL, following its redirects. Each probe
   reads a 512 KiB sample with a 10-second overall deadline. The response is
   dropped at that boundary even if the server ignores the Range header.
3. Finish every probe, exclude failures, then rank successful providers by the
   total time to deliver the same sample. Report time to first byte and effective
   bytes per second. This is a current network estimate, not a guarantee of
   sustained multi-gigabyte throughput.
4. Download missing files from the fastest candidate. Stream to a unique temporary
   file in the destination directory and validate both size and SHA-256 before
   installing it atomically. Errors drop the temporary file. Transfer or checksum
   failures fall back to the next measured provider; local I/O failures stop.
5. Never overwrite another process's destination file. If a concurrent startup
   installs it first, verify that file before accepting it.

`DownloadBackend` resolves a model file to its provider URL. `Downloader` owns the
shared benchmark, verification and installation behavior. `RankedBackends` is
created only after the benchmark phase has completed. Further providers implement
the same trait; they do not copy download or integrity logic.

The built-in repositories are `openai/privacy-filter` on Hugging Face and
`openai-mirror/privacy-filter` on ModelScope. Each uses a pinned commit, and both
serve the same checked artifact contents. URL formats follow the
[Hugging Face download API](https://huggingface.co/docs/huggingface_hub/guides/download)
and [ModelScope client](https://github.com/modelscope/modelscope/blob/master/modelscope/hub/file_download.py).

## Router commands

```sh
# Startup prepares ./models/privacy-filter relative to the launch directory.
privacy-router

# Measure both providers before downloading missing files; do not start a server.
privacy-router model download

# Just show current provider latency/speed and the preferred source.
privacy-router model benchmark

# Use a specific source, or verify all files without any network access.
privacy-router --model-source modelscope model download
privacy-router --model-dir ./models/privacy-filter model verify
privacy-router --offline
```

`--model-source` / `PRIVACY_ROUTER_MODEL_SOURCE` can select `huggingface` or
`modelscope`; omit it to benchmark both. `--model-dir` /
`PRIVACY_ROUTER_MODEL_DIR` changes the destination. Offline startup verifies the
same pinned release and fails if files are missing or different. Explicitly
requesting a benchmark with `--offline` is an error.

Bootstrap shows terminal spinners for verification, storage setup and model
loading, and a byte counter, speed and ETA during downloads. Bars are hidden for
redirected stderr and JSON logging. Structured events remain available in logs.

The legacy `scripts/download_model.py` is a compatibility wrapper around the Rust
command. It no longer owns revisions, file lists, checksums or HTTP behavior.

## Verification

```sh
cargo test -p privacy-model --features download --test download
```

Tests use actual loopback HTTP servers and isolated temporary directories. They
exercise provider ranking, the preflight/download phase boundary, unavailable
and truncated samples, timeout handling, transfer fallback, file integrity,
offline reuse and atomic publication. They do not contact the public hubs.
