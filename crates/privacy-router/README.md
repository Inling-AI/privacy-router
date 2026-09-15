# privacy-router

Release binaries include both inference backends: CPU is the default; use
`--backend gpu` or `PRIVACY_ROUTER_BACKEND=gpu` for GPU inference. See the
[binary release guide](../../docs/releases.md) for platform requirements.

The application binary and HTTP layer. Runs local privacy detection, applies redaction rules, forwards LLM requests, and serves the Flutter console.

## Run from Source

From the workspace root:

```sh
mise run setup
mise run app:build:release:cpu
./target/release/privacy-router
```

Open `http://127.0.0.1:8787`. The first visit is a setup form: create the single administrator account there and the console opens immediately. Setup is accepted only from the machine running the proxy, so create the account while it is bound to a loopback address (the default) rather than after exposing it.

Add an upstream in the console with its protocol and base URL, such as `https://api.example.com/v1`. Configure your LLM client to use `http://127.0.0.1:8787/v1` with its usual upstream credentials.

`privacy-router init-admin` remains for headless hosts and for rotating credentials; `--force` replaces the existing account and invalidates every session.

Use `--database` and `--model-dir` to select storage and model weights. Run `privacy-router --help` for all flags and environment variables. Public binding requires `--allow-public-bind`; deploy TLS and network access controls before exposing the service.

Startup verifies the pinned model in `./models/privacy-filter` (relative to the
launch directory). When files are missing, it first tests Hugging Face and
ModelScope latency and transfer speed, waits for all probes, then downloads from
the fastest successful provider. Downloads are checksum-verified and installed
atomically, with fallback to the next measured provider on transfer failure.
`indicatif` displays bootstrap stages and download progress in an interactive
terminal. JSON logs and redirected stderr do not contain progress-bar output.

Use `privacy-router model download` to prepare files without starting the server,
`privacy-router model benchmark` to measure sources only, or `privacy-router model
verify` for offline verification. `--model-source huggingface|modelscope` pins a
source; omit it for automatic selection. `--offline` starts using verified local
files without networking. See the [model download contract](../privacy-model/README.md).

App tasks are named `<domain>:<action>:<form>[:<backend>]`. `app:run:dev` and `app:run:release` read build features from `PRIVACY_ROUTER_CARGO_FEATURES` and runtime selection from `PRIVACY_ROUTER_BACKEND`; the `:cpu` and `:gpu` variants pin those variables and delegate, so nothing about compilation or startup is duplicated between them.

For development, run `mise run app:run:dev:gpu` and `mise run web:dev` in separate terminals. Flutter accepts `r` for hot reload and `R` for restart. This form serves the console from the Dart dev server, so it opens `--dev-origin` for cross-origin requests and does not embed web assets.

For the packaged binary, `mise run app:run:release:cpu` and `mise run app:run:release:gpu` both go through `app:build:release`, which prepares the model, builds the web assets, and compiles the Rust binary with the required backend support before starting it. `mise run app:run:debug:cpu` runs the same development form as a debug build: inference is far too slow to interact with, so it only helps when troubleshooting something outside the model.

The packaged binary embeds a pruned console build: `web:build` deletes the dart2js fallback and the canvaskit runtime that only it uses, and no longer ships DevTools symbol files. The console therefore needs a browser with WasmGC support (Chrome/Edge 119+, Firefox 120+, Safari 18.2+); the loader fails before fetching anything on browsers without it. Rationale and the exact deletion list live in [scripts/prune_web_release.sh](../../scripts/prune_web_release.sh).

## Request Handling

POST requests to `/v1/chat/completions`, `/v1/responses`, and `/v1/messages` enter the redaction pipeline. It scans supported message text, system instructions, and tool-result text. Nothing the model itself wrote is scanned: assistant messages, tool-call arguments, and reasoning summaries were passed through by us in the first place, so re-reading them cannot stop a new leak and only makes the context the upstream sees differ from what the model actually wrote. Images are excluded as well. Known `exec` custom output wrappers are unpacked and reconstructed around the processed output text.

Other methods and `/v1/*` routes pass through without scanning. Client credentials are forwarded. `x-privacy-router-provider` explicitly selects an upstream and is removed before forwarding; otherwise selection uses enabled providers and the request protocol. Upstream response bodies, including SSE streams, pass through without redaction.

Parse and inference failures reject processed requests. Audit-write failures are logged without undoing redaction. The default unmatched-entity policy is release and can be changed in the console. Detection is probabilistic; only identified spans receive rule decisions.

Detection is probabilistic, so the same text can be released in one turn and redacted in another. The content pool therefore aggregates by the exact text rather than by request: each row is one piece of content with its whole decision history, flipping rows first. Registering a direction there rewrites that single registration, and it applies to every occurrence of that text from then on.

## Working on the Request Path

| Code | Responsibility |
| --- | --- |
| `server.rs`, `proxy.rs` | Route dispatch, provider selection, HTTP forwarding |
| `protocol/`, `pipeline.rs` | Text extraction, classification, rule evaluation, writeback |
| `inference.rs`, `redaction.rs` | Prediction caching, overlapping windows, span replacement |
| `prefix.rs` | Persistent turn hashes, prefix reuse, and prediction writeback |
| `console.rs` | Authenticated administration API |

Keep forwarding tests focused on the payload actually received by an upstream. Protocol edits must preserve unrelated fields and tool-call structure. A batch containing oversized text currently uses sequential fallback; latency measurements can also include inference-lock waiting.

Completed turns are persisted as parent-linked hashes with their entity predictions. Replays reuse those predictions after restart; appended turns add nodes and edited history branches. Model files, backend, and inference options identify the prediction namespace. Current rules are evaluated on every request, so replayed turns are still decided and redacted under the rules in force now. The first request after enabling this storage must populate its chain. Model weights are still loaded at startup.

Deterministic filters run alongside the model. `FilteredClassifier` wraps the end of the recognition chain and hands model entities to `privacy_filter::Filters::combine`, which decides by category: an authoritative filter's category takes only its own hits, so an email is reported whether the model saw it or not and a permission string the model mistook for one never reaches the rules; an additive filter's category keeps the model's hits outside the ones the pattern already found. Replayed and freshly inferred content go through the same path.

The content pool only records content audited for the first time. A request resends its whole history, so replayed turns would otherwise add an entry on every request and bury the new turn; only the appended turn is new material.

Each span carries one way back: a redacted finding can be released, a released one denied. Both register an ordinary rule over that exact text — release at the release-keyword layer, deny at the force-redact layer above it — so denying a span that was released earlier wins on the next request, and either rule can be edited or deleted like any other.

The audit database retains original sensitive text, including entity values in persisted nodes. Protect the database and backups. Prefix nodes currently remain until the database is removed; automatic retention is not implemented. Real-request checks use the [relay tasks](../../scripts/relay.md); keep payloads and credentials out of version control.

### GPU memory reproduction (macOS)

Inference is serialized on a stable Burn stream. A mutex alone does not keep GPU
work on the same stream: Tokio's blocking pool can change the calling thread,
and CubeCL retains a separate device memory pool for each stream.

From the workspace root, run the real GPU router against an isolated loopback
upstream and temporary database:

```sh
cargo build --release -p privacy-router --features gpu
python3 scripts/gpu_memory_soak.py --rounds 4 --concurrency 16 --repeat 24 --vary 64 --fields 8 --log /tmp/privacy-gpu-soak.log
```

The probe checks forwarded email redaction and records a cold-start snapshot
before inference plus a snapshot after each round of requests. It reports both
RSS and physical footprint; on macOS, substantial Metal allocations appear as
unmapped graphics memory in the latter and are absent from ordinary RSS.
Snapshots are saved next to the log as `privacy-gpu-soak.<completed>.footprint.txt`,
with allocation categories in bytes. The physical-footprint ceiling is checked
after each round, so an in-progress round can exceed it. The probe terminates its
router and upstream when finished. It requires permission to access Metal and
inspect its child process with `footprint`.

Measured on macOS/Metal on 2026-09-15 with the workload above (GiB):

| Completed requests | Previous physical footprint | Stable-stream physical footprint |
| --- | ---: | ---: |
| 0 (cold start) | 8.51 | 7.01 |
| 16 | 13.56 | 7.72 |
| 32 | 17.76 | 7.58 |
| 48 | 21.78 | 7.61 |
| 64 | 26.38 | 6.84 |

Both runs started with 5.89 GiB in `Owned physical footprint (unmapped)
(graphics)`. That category grew to 24.78 GiB previously; with a stable stream it
reached 6.36 GiB by request 32 and remained there through request 64. Host malloc
residency varied between runs, so the graphics category is the more direct
comparison of device retention. All 64 requests in each run reached the fake
upstream with email redaction verified. These measurements establish the fix for
this workload on Metal; CUDA and Vulkan have not been measured.
