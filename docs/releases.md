# Privacy Router binary releases

Download the executable for your operating system and CPU architecture. Each
executable includes both CPU and GPU inference backends and the embedded web
console. Model weights are downloaded and verified on first startup; they are not
included in the executable. Usage documentation and the Apache-2.0 license are
available in the repository.

## Platform and backend availability

| Executable target | Included CPU backend | Included GPU backend | Build environment |
| --- | --- | --- | --- |
| Windows x64 (`x86_64-pc-windows-msvc`) | Burn NdArray | WGPU / Vulkan | Windows Server 2025 |
| Linux x64 (`x86_64-unknown-linux-gnu`) | Burn NdArray | WGPU / Vulkan | Ubuntu 22.04 (glibc 2.35) |
| macOS Intel (`x86_64-apple-darwin`) | Burn NdArray | WGPU / Metal | macOS 15 Intel |
| macOS Apple Silicon (`aarch64-apple-darwin`) | Burn NdArray | WGPU / Metal | macOS 15 ARM64 |

CPU inference does not require GPU drivers. GPU inference needs a compatible GPU and
its installed drivers: a Vulkan implementation on Windows/Linux, or Metal on
macOS. Windows/Linux executables are **not CUDA or ROCm builds**; NVIDIA, AMD, or
Intel hardware must be usable through Vulkan. This release uses Burn 0.21's WGPU
backend with fusion, the WGSL compiler on Windows/Linux, and native MSL on macOS.
The binary defaults to CPU. Select `--backend gpu` (or set
`PRIVACY_ROUTER_BACKEND=gpu`) to use the GPU. Explicit GPU selection does not
automatically fall back to CPU if device initialization fails; restart with
`--backend cpu` if needed.

Linux executables target glibc-based distributions at least as new as Ubuntu 22.04;
Alpine/musl is not a supported binary target. Windows executables may require the
Microsoft Visual C++ 2015–2022 x64 Redistributable. macOS executables are built on
macOS 15 and are unsigned and not notarized. Windows executables are not Authenticode
signed. Other architectures and older OS versions are not validated by this CI.

CI checks the CPU-only feature configuration, then builds, lints, and runs the
workspace tests with GPU support enabled on each native OS/architecture. It also
checks each release binary's `--help` and `--version`.
The model tests use the committed tiny fixture on CPU. **Passing the GPU build
job does not certify GPU inference on physical hardware**: hosted runners are not
used for hardware inference validation. GPU driver compatibility, numerical
behavior, memory capacity, and throughput still need testing on the target GPU.

These backend choices follow the pinned dependency implementation:
[Burn WGPU](https://docs.rs/burn-wgpu/0.21.0/burn_wgpu/) and
[CubeCL graphics selection](https://docs.rs/cubecl-wgpu/0.10.0/src/cubecl_wgpu/graphics.rs.html).

## Run

Save the executable in a writable directory. Rename it to `privacy-router` on
macOS/Linux (or `privacy-router.exe` on Windows). On macOS/Linux, run
`chmod +x privacy-router` first, then:

```sh
./privacy-router --backend cpu
# Or use a compatible GPU with the same executable:
./privacy-router --backend gpu
```

On Windows PowerShell, use `.\privacy-router.exe`.
Open <http://127.0.0.1:8787> and create the administrator account in the console.
The default listener is local to your machine. The executable creates its database
and model files relative to the current working directory.

For an offline machine, prepare the model on a connected machine first:

```sh
./privacy-router model download
./privacy-router --offline
```

Copy the downloaded `models/privacy-filter` directory to the offline machine, or
point `--model-dir` to its location. Run `--help` for all configuration options.

## Integrity

GitHub records a SHA-256 digest for each uploaded release asset. Compare that digest
with a locally computed hash: `sha256sum <executable>` on Linux,
`shasum -a 256 <executable>` on macOS, or
`Get-FileHash <executable> -Algorithm SHA256` in PowerShell. The release asset API
exposes the digest in the `digest` field. There is no separate web bundle or checksum
file among the downloadable assets.

## CI and publishing

`.github/workflows/ci.yml` runs on pull requests, pushes to `master`, manual
workflow dispatches, and `v*` tags. It checks the Flutter console, then each native job builds its own web assets using
`.fvmrc` and embeds them directly. Web assets are never uploaded as workflow artifacts. Rust uses
`rust-toolchain.toml`; dependency resolution is locked. Only native executables are retained as
workflow artifacts even for builds that do not publish a release.

To publish, set the workspace version in `Cargo.toml` (and update internal crate
version constraints and lockfiles as needed), then push a matching `v<version>`
tag. A version mismatch fails executable staging. Only after every matrix job passes does
the release job upload the platform executables to a draft and publish it.
The workflow uses `GITHUB_TOKEN`; no personal access token is needed.
