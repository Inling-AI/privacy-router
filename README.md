# Privacy Router

[![CI](https://github.com/Inling-AI/privacy-router/actions/workflows/ci.yml/badge.svg)](https://github.com/Inling-AI/privacy-router/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Inling-AI/privacy-router)](https://github.com/Inling-AI/privacy-router/releases/latest)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

A local privacy proxy for LLM applications. A dedicated recognition model finds
sensitive content in real requests and collects those findings in a reviewable
content pool. It does not use a generative LLM to review your prompts.

Inspect a finding's occurrences and original/forwarded text, then **deny** a
sensitive credential or **release** an allowed value or false positive. Your
decision becomes a rule: subsequent occurrences of the same text follow that
decision, without writing a rule by hand. Recognition runs on your machine;
registered decisions are stored as policy, not used to retrain the model.

**Download one executable. Run on CPU or GPU. No Python environment, external
database, or separate frontend server is required.**

## Features

- Local model inference plus deterministic filters for supported entity categories.
- Request processing for OpenAI Chat Completions, OpenAI Responses, and Anthropic Messages.
- Rules for releasing or redacting detected content, with an auditable content pool.
- Streaming upstream responses and pass-through of client credentials.
- Persistent inference caching and reuse of previously processed conversation turns.
- Embedded web console, SQLite storage, and automatic verified model downloads.
- Runtime CPU/GPU selection in the same release executable.

## Interactive demo

[Open the static demo](https://inling-ai.github.io/privacy-router-demo/) and select
**Enter demo** with the prefilled `demo` / `demo` credentials.
Explore sample statistics, review detected content, and edit rules and upstreams.
The demo runs entirely in your browser: it has no backend, performs no inference,
and never connects to the configured providers. All metrics and content are
fictional samples. Edits reset on reload or **Reset demo**; the demo sign-in is
remembered locally for 12 hours. Do not enter real credentials.

The [demo workflow](.github/workflows/demo.yml) validates and builds demo changes
on `master`. The public [demo repository](https://github.com/Inling-AI/privacy-router-demo)
contains only compiled assets; GitHub Pages serves its `main` branch.
To publish an update using an authenticated Git checkout, run
`bash scripts/publish_demo.sh`. Cross-repository publishing is manual because
the organization disables repository deploy keys. To run it locally:

```bash
cd apps/console
fvm flutter pub get
fvm flutter run -d chrome --dart-define=PRIVACY_ROUTER_DEMO=true
```

For static hosting, build with `fvm flutter build web --wasm --release
--dart-define=PRIVACY_ROUTER_DEMO=true --base-href /privacy-router-demo/
--output build/demo` (as one command), then serve `build/demo`. Set `--base-href`
to your site's path, including its leading and trailing slash. Demo output is
separate from the production console embedded in release executables.

## Download

Get the executable for your machine from **[GitHub Releases](https://github.com/Inling-AI/privacy-router/releases/latest)**:

| Your machine | Executable name ends with |
| --- | --- |
| Windows, Intel/AMD 64-bit | `x86_64-pc-windows-msvc.exe` |
| Linux, Intel/AMD 64-bit | `x86_64-unknown-linux-gnu` |
| macOS, Apple Silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |

Every executable includes both CPU and GPU inference. CPU is the default and does
not require GPU drivers. GPU inference uses Vulkan on Windows/Linux and Metal on
macOS; these are not CUDA-specific packages. See the [release guide](docs/releases.md)
for OS requirements, GPU validation limits, signing status, and integrity verification.

The console needs a browser with WasmGC support. Release executables do not include
the model: first startup downloads roughly 2.8 GB of weights plus tokenizer and
configuration files. Allow additional disk space for the database and sufficient
RAM or GPU memory for model execution; requirements grow with input length and
cache limits. The download size is not the runtime memory requirement.

## Quick start

1. Save the executable in a writable directory. Rename it to `privacy-router`
   (macOS/Linux) or `privacy-router.exe` (Windows). On macOS/Linux, make it
   executable with `chmod +x privacy-router`.
2. Start the executable from that directory:

   **macOS / Linux**

   ```sh
   ./privacy-router
   ```

   **Windows PowerShell**

   ```powershell
   .\privacy-router.exe
   ```

3. Wait for model verification/download and loading to finish, then open
   **<http://127.0.0.1:8787>**.
4. Create the administrator account on the first-run setup page.
5. Add an upstream in **Upstreams**, selecting its protocol and base URL.
6. Point your LLM client at `http://127.0.0.1:8787/v1`, keeping the API credentials
   normally used with that upstream.

For example, after configuring an OpenAI-compatible upstream:

```sh
curl http://127.0.0.1:8787/v1/chat/completions \
  -H "Authorization: Bearer $UPSTREAM_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "your-upstream-model",
    "messages": [{"role": "user", "content": "Please email alice@example.com."}]
  }'
```

Use a model name accepted by your upstream. Review the rule policy in the console
before sending sensitive requests: detecting an entity does not necessarily mean
it will be redacted. Rules determine the outcome.

The administrator login protects the console. It is separate from credentials
forwarded to upstream providers. To select a particular configured upstream, add
`x-privacy-router-provider: <provider-name>` to your request.

## CLI usage

Running without a subcommand starts the proxy and console. Put server options
before a subcommand:

```sh
privacy-router [OPTIONS] [COMMAND]
privacy-router --help
privacy-router model --help
privacy-router init-admin --help
```

In examples below, use `./privacy-router` on Unix or `.\privacy-router.exe` in
PowerShell when the executable is not on your `PATH`.

### Choose CPU or GPU

```sh
privacy-router --backend cpu
privacy-router --backend gpu
```

Alternatively set `PRIVACY_ROUTER_BACKEND=cpu` or `gpu`. GPU selection is explicit:
if GPU initialization fails, restart with `--backend cpu`. Release binaries contain
both backends; a source build without the `gpu` Cargo feature accepts CPU only.

### Choose storage and listener

```sh
privacy-router \
  --bind 127.0.0.1:8787 \
  --database ./data/privacy-router.db \
  --model-dir ./models/privacy-filter
```

Create `./data` first. Relative paths resolve from your current working directory.
Use a stable working directory or absolute paths when running as a service.

### Download, verify, or use a model offline

```sh
# Prepare model files without starting the server.
privacy-router model download

# Inspect source connectivity and transfer speed.
privacy-router model benchmark

# Select a source explicitly instead of automatic source selection.
privacy-router --model-source huggingface model download
privacy-router --model-source modelscope model download

# Verify local files without network access.
privacy-router model verify

# Start with verified local model files; disable model downloads.
privacy-router --offline
```

For an offline host, download on a connected machine and copy the model directory
across. Use `--model-dir` if its location differs. `--offline` controls model
preparation; it does not disable forwarding to network-based upstream providers.
The downloader checks pinned sizes and SHA-256 hashes. See the
[model download contract](crates/privacy-model/README.md) for failure and fallback behavior.

### Manage administrator credentials

The web setup page is the simplest first-run path. For a headless host:

```sh
privacy-router init-admin --username admin

# Replace the existing account and revoke all active console sessions.
privacy-router init-admin --username admin --force
```

The command prompts for a password. `--password` and
`PRIVACY_ROUTER_ADMIN_PASSWORD` are also supported; avoid putting secrets directly
in shell history or process arguments. Create the administrator locally before
exposing the listener to other machines.

### Tune resource limits and logging

```sh
privacy-router \
  --max-tokens 4096 \
  --cache-entries 1024 \
  --cache-bytes 134217728 \
  --max-body-bytes 8388608 \
  --upstream-timeout-seconds 300 \
  --session-ttl-seconds 43200 \
  --log-format json \
  --log-filter info \
  --log-dir ./logs
```

These are example settings, not a statement of defaults. `--help` is generated
from the CLI types and lists the current defaults, accepted values, and environment
variables. Server environment variables use the `PRIVACY_ROUTER_` prefix, including
`BIND`, `DATABASE`, `MODEL_DIR`, `BACKEND`, `MAX_TOKENS`, `CACHE_ENTRIES`, and
`CACHE_BYTES`. Explicit arguments take precedence over environment values.

### Bind to a network interface

```sh
privacy-router --bind 0.0.0.0:8787 --allow-public-bind
```

Do this only behind appropriate network access controls and TLS termination.
Public binding is an explicit opt-in. The proxy routes do not gain console-session
authentication merely because the console has an administrator account.

## What is processed—and what is stored

- POST requests to `/v1/chat/completions`, `/v1/responses`, and `/v1/messages` enter
  the privacy pipeline. Supported user text, system instructions, and tool-result
  text are scanned.
- Assistant-generated text, tool-call arguments, reasoning summaries, and images
  are not scanned. Other `/v1/*` routes pass through without privacy processing.
- Upstream responses, including streamed responses, pass through without redaction.
- Client credentials are forwarded; the proxy does not save or inject upstream API keys.
- Parse or inference failures reject requests being processed. Detection remains
  probabilistic, and rule decisions only apply to identified content.
- **The local audit database retains original sensitive text.** Protect the database,
  its directory, and backups. Persistent prefix records have no automatic retention
  policy yet.

Local inference reduces what needs to leave your machine; it does not guarantee
that every piece of sensitive information will be detected or blocked. Review your
rules and test representative requests before relying on a policy.

## Build from source

Toolchain versions are pinned in [rust-toolchain.toml](rust-toolchain.toml) and
[.fvmrc](.fvmrc). The [mise tasks](mise.toml) document the local development workflow.
With Rust, Flutter/FVM, and the native compiler tools for your platform installed:

```sh
# Build the assets embedded by Rust. Run from the repository root.
cd apps/console
fvm flutter pub get --enforce-lockfile
fvm flutter build web --wasm --release
cd ../..
bash scripts/prune_web_release.sh apps/console/build/web

# Include both inference backends in the executable.
cargo build --locked --release -p privacy-router --features gpu
./target/release/privacy-router --backend cpu
```

Use Git Bash for the shell commands on Windows; the executable is
`target/release/privacy-router.exe`. Omit `--features gpu` for a CPU-only source
build. Model downloads happen at runtime and are not required to compile.

For development with a separately served console:

```sh
mise run setup
mise run app:run:dev:gpu
# In a second terminal:
mise run web:dev
```

The development tasks configure loopback CORS for the Flutter server. Use
`app:run:dev:cpu` when GPU inference is not wanted.

### Checks

Build the web assets first, then run:

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --features gpu -- -D warnings
cargo test --locked --workspace --features gpu
cd apps/console
fvm flutter analyze
fvm flutter test
```

[GitHub Actions](.github/workflows/ci.yml) checks the console, compiles native builds
for every release target, runs Rust tests, and uploads only the platform executables.
Matching `v<version>` tags publish to GitHub Releases after the matrix succeeds.
GPU compilation and CLI smoke tests do not replace inference testing on real GPUs.

## Project structure

| Component | Purpose |
| --- | --- |
| [privacy-router](crates/privacy-router) | Executable, HTTP proxy, console API, application orchestration |
| [privacy-model](crates/privacy-model) | Pinned model artifacts, download providers, integrity verification |
| [privacy-filter](crates/privacy-filter) | Tokenization, model execution, decoding, deterministic filters |
| [privacy-filter-cache](crates/privacy-filter-cache) | In-memory prediction caching |
| [privacy-rules](crates/privacy-rules) | Privacy rule evaluation |
| [privacy-store](crates/privacy-store) | SQLite persistence and migrations |
| [privacy-telemetry](crates/privacy-telemetry) | Logging and telemetry configuration |
| [apps/console](apps/console) | Embedded Flutter web console |

## Troubleshooting

- **GPU startup fails:** try `--backend cpu`. On Windows/Linux, check Vulkan driver
  availability; a CUDA toolkit alone does not provide the required backend.
- **Model download fails:** run `model benchmark`, try an explicit `--model-source`,
  or prepare the model elsewhere and copy it locally. `model verify` checks the files.
- **Out of memory:** reduce `--max-tokens` and cache limits; GPU inference also needs
  enough device or unified memory. See [measured Metal behavior](crates/privacy-router/README.md#gpu-memory-reproduction-macos).
- **Console will not load:** use a modern WasmGC-capable browser and the URL printed
  at startup. Source builds must include the web assets before Rust compilation.
- **Requests are rejected or choose the wrong upstream:** check the upstream protocol,
  base URL, enabled state, and optional `x-privacy-router-provider` header.
- **Forgotten administrator password:** run `init-admin --force` against the same database.

## Contributing and reporting issues

Bug reports and focused pull requests are welcome. Include your OS, CPU architecture,
application version, selected backend, GPU/driver details if applicable, reproduction
steps, and sanitized logs. Never attach real API keys, private conversations, or
an audit database containing sensitive data.

Follow [AGENTS.md](AGENTS.md). Rust behavior tests belong in each crate's `tests/`
directory and should exercise public contracts. Include relevant verification with
a change, and document user-visible behavior or CLI changes.

Use [Issues](https://github.com/Inling-AI/privacy-router/issues) for reproducible bugs
and feature requests. Do not disclose exploitable vulnerabilities or secrets in
public issues; use private vulnerability reporting if enabled for the repository.

## Community

Thanks to the [LINUX DO](https://linux.do/) community for providing a place to
share and discuss open-source projects. Feedback, bug reports, and contributions
from the community are welcome.

## License

Privacy Router is licensed under [Apache-2.0](LICENSE). Third-party dependencies and
model artifacts retain their respective licenses; consult their upstream notices
when redistributing them.
