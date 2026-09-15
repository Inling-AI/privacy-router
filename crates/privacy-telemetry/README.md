# privacy-telemetry

Privacy Router's logging setup. `main` installs `Telemetry` once and retains `TelemetryGuard` until application tasks stop. Configuration chooses human-readable or JSON output, filter directives, and optional daily rolling files.

`src/lib.rs` builds the subscriber and owns file-worker shutdown. `tee.rs` sends the same formatted output to stderr and the file writer. File writes are buffered on a worker; stderr writes are synchronous. Retention is managed outside the application.

Changes must preserve guard-based flushing and avoid installing additional global subscribers. Embedded callers and tests can construct a subscriber with `Telemetry::subscriber`. Formatting performs no redaction: log request IDs and aggregate measurements, never credentials, prompts, or entity text.
