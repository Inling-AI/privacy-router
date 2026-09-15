# Relay Tests

The three tasks accept a payload file. They use `uv` with the locked dependencies
in `relay.py.lock`; no OpenAI or Anthropic SDK is required.

```sh
export RELAY_TEST_KEY='<upstream test key>'
mise run relay:chat path/to/chat.json
mise run relay:responses path/to/responses.json
mise run relay:messages path/to/messages.json
```

The script preserves the payload's model, history, tools and extension fields,
then appends one user message asking the model to echo a synthetic name, email
and phone number. It never edits the input file. The returned text shows what
reached the model after processing; this is a smoke test, not proof that every
sensitive field in the history was scanned. Tool calls are never executed.

The reply is written to stdout. HTTP status, elapsed time and failures go to
stderr. Failed requests, incomplete streams and replies without text exit
nonzero. Both SSE and non-streaming JSON are supported.

Optional environment variables:

- `PRIVACY_ROUTER_RELAY_URL`: defaults to `http://127.0.0.1:8787/v1`.
- `PRIVACY_ROUTER_PROVIDER`: selects a configured upstream by name.
- `RELAY_TEST_MESSAGE`: replaces the appended probe message.

Chat and Responses use `Authorization: Bearer`; Messages uses `x-api-key` and
`anthropic-version`. No key is stored in the script, task or provider config.

Run the GPU development server with `mise run app:run:dev:gpu`. Its startup log must
identify the actual GPU adapter. This task shares all other settings with
`app:run:dev` and keeps the normal interactive foreground process.
