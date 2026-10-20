#!/usr/bin/env python3
"""Fetch and normalize the OpenCode Zen model catalog."""

import argparse
import json
import os
from pathlib import Path
import sys
import tempfile
import urllib.error
import urllib.request

MODELS_URL = "https://opencode.ai/zen/v1/models"
USER_AGENT = "opencode-update-skill"
AUTH_FILE = Path.home() / ".local/share/opencode/auth.json"
TIMEOUT_SECONDS = 30


class CatalogError(Exception):
    """Raised when the catalog cannot be fetched or validated."""


def resolve_key(environment=None, auth_file=None):
    environment = os.environ if environment is None else environment
    auth_file = AUTH_FILE if auth_file is None else Path(auth_file)
    key = environment.get("OPENCODE_API_KEY", "").strip()
    if key:
        return key
    try:
        auth = json.loads(auth_file.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise CatalogError(
            f"Set OPENCODE_API_KEY or provide an OpenCode key in {auth_file}."
        ) from error
    opencode = auth.get("opencode") if isinstance(auth, dict) else None
    key = opencode.get("key") if isinstance(opencode, dict) else None
    if not isinstance(key, str) or not key.strip():
        raise CatalogError(
            f"Set OPENCODE_API_KEY or provide an OpenCode key in {auth_file}."
        )
    return key.strip()


def fetch_catalog(key, url=MODELS_URL, opener=urllib.request.urlopen):
    request = urllib.request.Request(
        url,
        headers={
            "Authorization": f"Bearer {key}",
            "User-Agent": USER_AGENT,
        },
    )
    try:
        with opener(request, timeout=TIMEOUT_SECONDS) as response:
            return json.load(response)
    except urllib.error.HTTPError as error:
        raise CatalogError(
            f"Model catalog request failed with HTTP status {error.code}."
        ) from error
    except (urllib.error.URLError, TimeoutError) as error:
        raise CatalogError("Model catalog request failed.") from error
    except ValueError as error:
        raise CatalogError("Model catalog response is not valid JSON.") from error


def normalize(payload):
    if not isinstance(payload, dict) or not isinstance(payload.get("data"), list):
        raise CatalogError("Model catalog response does not contain a data list.")
    entries = payload["data"]
    if not entries:
        raise CatalogError("Model catalog response is empty.")
    models = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise CatalogError("Model catalog contains a non-object entry.")
        model_id = entry.get("id")
        if not isinstance(model_id, str) or not model_id.strip():
            raise CatalogError("Model catalog contains an entry without a string id.")
        model_id = model_id.strip()
        if any(character.isspace() for character in model_id):
            raise CatalogError(f"Model catalog id contains whitespace: {model_id!r}.")
        models.add(model_id if "/" in model_id else f"opencode/{model_id}")
    return sorted(models)


def write_models(path, models):
    content = "\n".join(models) + "\n"
    if path is None:
        sys.stdout.write(content)
        return
    destination = Path(path)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            "w",
            encoding="utf-8",
            dir=destination.parent,
            prefix=".opencode-models-",
            delete=False,
        ) as handle:
            handle.write(content)
            temporary = Path(handle.name)
        os.replace(temporary, destination)
    except OSError as error:
        if temporary is not None:
            try:
                temporary.unlink()
            except OSError:
                pass
        raise CatalogError(f"Cannot write {destination}.") from error


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output", type=Path, help="Write normalized model ids to this file."
    )
    args = parser.parse_args(argv)
    try:
        key = resolve_key()
        models = normalize(fetch_catalog(key))
        write_models(args.output, models)
    except CatalogError as error:
        print(error, file=sys.stderr)
        return 1
    print(f"Fetched {len(models)} models.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
