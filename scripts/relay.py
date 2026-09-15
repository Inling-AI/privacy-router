# /// script
# requires-python = ">=3.11"
# dependencies = ["httpx>=0.28,<0.29", "httpx-sse>=0.4,<0.5"]
# ///
"""Replay a real payload through the relay with one additional echo message."""

import argparse
import json
import os
import sys
import time
from enum import Enum
from pathlib import Path

import httpx
from httpx_sse import EventSource


class Protocol(Enum):
    CHAT = ("chat", "/chat/completions", "messages")
    RESPONSES = ("responses", "/responses", "input")
    MESSAGES = ("messages", "/messages", "messages")

    def prepare(self, body: dict, message: str) -> dict:
        result = dict(body)
        field = self.value[2]
        history = result.get(field, [])
        if self is Protocol.RESPONSES and isinstance(history, str):
            history = [{"role": "user", "content": history}]
        if not isinstance(history, list):
            raise ValueError(f"{field} must be a message list")
        result[field] = [*history, {"role": "user", "content": message}]
        return result

    def headers(self, key: str, provider: str | None) -> dict:
        headers = {"content-type": "application/json", "accept": "text/event-stream, application/json"}
        if self is Protocol.MESSAGES:
            headers.update({"x-api-key": key, "anthropic-version": "2023-06-01"})
        else:
            headers["authorization"] = f"Bearer {key}"
        if provider:
            headers["x-privacy-router-provider"] = provider
        return headers

    def reply(self, data: dict, streaming: bool) -> str:
        if self is Protocol.CHAT:
            return "".join(
                choice.get("delta" if streaming else "message", {}).get("content") or ""
                for choice in data.get("choices", [])
            )
        if self is Protocol.RESPONSES:
            if streaming:
                return data.get("delta", "") if data.get("type") == "response.output_text.delta" else ""
            return "".join(
                block.get("text", "")
                for item in data.get("output", []) if item.get("type") == "message"
                for block in item.get("content", []) if block.get("type") == "output_text"
            )
        if streaming:
            delta = data.get("delta", {})
            return delta.get("text", "") if delta.get("type") == "text_delta" else ""
        return "".join(block.get("text", "") for block in data.get("content", []) if block.get("type") == "text")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("protocol", choices=[p.value[0] for p in Protocol])
    parser.add_argument("payload", type=Path)
    parser.add_argument("--base-url", default=os.getenv("PRIVACY_ROUTER_RELAY_URL", "http://127.0.0.1:8787/v1"))
    parser.add_argument("--provider", default=os.getenv("PRIVACY_ROUTER_PROVIDER"))
    parser.add_argument("--message", default=os.getenv(
        "RELAY_TEST_MESSAGE",
        "Please repeat only the following text exactly as you received it. Do not use tools or add explanations:\n"
        "My name is Avery Morgan. My email is avery.morgan@example.net. My phone number is +1-415-555-0198.",
    ))
    args = parser.parse_args()
    key = os.getenv("RELAY_TEST_KEY")
    if not key:
        parser.error("set RELAY_TEST_KEY to the upstream test key")
    protocol = next(p for p in Protocol if p.value[0] == args.protocol)
    body = json.loads(args.payload.read_text(encoding="utf-8"))
    if not isinstance(body, dict) or not isinstance(body.get("model"), str):
        parser.error("payload must be a JSON object containing model")
    body = protocol.prepare(body, args.message)
    url = args.base_url.rstrip("/") + protocol.value[1]
    started = time.monotonic()
    print(f"{args.protocol}: model={body['model']} file={args.payload.name}", file=sys.stderr)
    received_text = False
    completed = False
    timeout = httpx.Timeout(connect=15, read=1800, write=60, pool=15)
    with httpx.Client(timeout=timeout) as client:
        with client.stream("POST", url, headers=protocol.headers(key, args.provider), json=body) as response:
            print(f"HTTP {response.status_code}; headers after {time.monotonic() - started:.2f}s", file=sys.stderr)
            if response.is_error:
                response.read()
                # Report the gateway error, never dump the request payload or headers.
                error = response.text.replace(key, "[credential]")
                raise RuntimeError(f"upstream HTTP {response.status_code}: {error[:1500]}")
            if "text/event-stream" in response.headers.get("content-type", ""):
                for event in EventSource(response).iter_sse():
                    if event.data == "[DONE]":
                        completed = True
                        continue
                    if not event.data:
                        continue
                    data = json.loads(event.data)
                    kind = data.get("type", event.event)
                    if kind in {"error", "response.failed", "response.incomplete"} or data.get("error"):
                        raise RuntimeError(f"stream failed: {json.dumps(data.get('error') or data.get('response', {}).get('error') or kind)}")
                    completed |= kind in {"response.completed", "message_stop"}
                    text = protocol.reply(data, streaming=True)
                    if text:
                        received_text = True
                        print(text, end="", flush=True)
            else:
                response.read()
                data = response.json()
                if data.get("error") or data.get("status") in {"failed", "incomplete"}:
                    raise RuntimeError(f"response failed: {json.dumps(data.get('error') or data.get('status'))}")
                text = protocol.reply(data, streaming=False)
                received_text = bool(text)
                print(text, end="", flush=True)
                completed = True
    print()
    print(f"Finished in {time.monotonic() - started:.2f}s", file=sys.stderr)
    if not completed:
        raise RuntimeError("stream closed without a completion event")
    if not received_text:
        raise RuntimeError("the model returned no text reply; tool calls are not executed by this test")


if __name__ == "__main__":
    try:
        main()
    except (httpx.HTTPError, OSError, ValueError, RuntimeError) as error:
        print(f"relay test failed: {error}", file=sys.stderr)
        sys.exit(1)
