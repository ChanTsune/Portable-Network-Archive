import io
import json
from contextlib import redirect_stderr
from pathlib import Path
import tempfile
import unittest
import urllib.error
from unittest.mock import patch

import fetch_model_catalog as catalog


class Response(io.BytesIO):
    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


class ResolveKeyTests(unittest.TestCase):
    def test_environment_key_wins(self):
        with tempfile.TemporaryDirectory() as directory:
            auth = Path(directory) / "auth.json"
            auth.write_text(
                json.dumps({"opencode": {"key": "file-key"}}), encoding="utf-8"
            )
            self.assertEqual(
                catalog.resolve_key({"OPENCODE_API_KEY": "env-key"}, auth), "env-key"
            )

    def test_auth_file_key_is_used(self):
        with tempfile.TemporaryDirectory() as directory:
            auth = Path(directory) / "auth.json"
            auth.write_text(
                json.dumps({"opencode": {"key": "file-key"}}), encoding="utf-8"
            )
            self.assertEqual(catalog.resolve_key({}, auth), "file-key")

    def test_missing_key_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(catalog.CatalogError):
                catalog.resolve_key({}, Path(directory) / "auth.json")

    def test_invalid_auth_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            auth = Path(directory) / "auth.json"
            auth.write_text("not json", encoding="utf-8")
            with self.assertRaises(catalog.CatalogError):
                catalog.resolve_key({}, auth)

    def test_auth_file_with_non_object_provider_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            auth = Path(directory) / "auth.json"
            auth.write_text(
                json.dumps({"opencode": "not-an-object"}), encoding="utf-8"
            )
            with self.assertRaises(catalog.CatalogError):
                catalog.resolve_key({}, auth)


class NormalizeTests(unittest.TestCase):
    def test_bare_ids_are_prefixed_and_sorted(self):
        self.assertEqual(
            catalog.normalize({"data": [{"id": "z-free"}, {"id": "opencode/a-free"}]}),
            ["opencode/a-free", "opencode/z-free"],
        )

    def test_non_list_data_is_rejected(self):
        with self.assertRaises(catalog.CatalogError):
            catalog.normalize({"data": {}})

    def test_empty_data_is_rejected(self):
        with self.assertRaises(catalog.CatalogError):
            catalog.normalize({"data": []})

    def test_non_string_id_is_rejected(self):
        with self.assertRaises(catalog.CatalogError):
            catalog.normalize({"data": [{"id": 1}]})

    def test_id_whitespace_is_rejected(self):
        with self.assertRaises(catalog.CatalogError):
            catalog.normalize({"data": [{"id": "bad id"}]})


class FetchCatalogTests(unittest.TestCase):
    def test_authorization_header_contains_key(self):
        captured = {}

        def opener(request, timeout):
            captured["request"] = request
            return Response(b'{"data": [{"id": "a-free"}]}')

        catalog.fetch_catalog("secret-key", opener=opener)
        self.assertEqual(
            captured["request"].get_header("Authorization"), "Bearer secret-key"
        )
        self.assertEqual(
            captured["request"].get_header("User-agent"), "opencode-update-skill"
        )

    def test_http_error_is_reported(self):
        def opener(request, timeout):
            raise urllib.error.HTTPError(
                request.full_url, 401, "Unauthorized", {}, io.BytesIO(b"{}")
            )

        with self.assertRaises(catalog.CatalogError):
            catalog.fetch_catalog("secret-key", opener=opener)

    def test_invalid_json_is_rejected(self):
        def opener(request, timeout):
            return Response(b"not json")

        with self.assertRaises(catalog.CatalogError):
            catalog.fetch_catalog("secret-key", opener=opener)


class MainTests(unittest.TestCase):
    def test_main_writes_normalized_catalog(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "models.txt"
            with (
                patch("fetch_model_catalog.resolve_key", return_value="secret-key"),
                patch(
                    "fetch_model_catalog.fetch_catalog",
                    return_value={"data": [{"id": "b-free"}, {"id": "a-free"}]},
                ),
                redirect_stderr(io.StringIO()),
            ):
                self.assertEqual(catalog.main(["--output", str(path)]), 0)
            self.assertEqual(
                path.read_text(encoding="utf-8"),
                "opencode/a-free\nopencode/b-free\n",
            )

    def test_main_leaves_output_unchanged_on_fetch_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "models.txt"
            path.write_text("old\n", encoding="utf-8")
            with (
                patch("fetch_model_catalog.resolve_key", return_value="secret-key"),
                patch(
                    "fetch_model_catalog.fetch_catalog",
                    side_effect=catalog.CatalogError("boom"),
                ),
                redirect_stderr(io.StringIO()),
            ):
                self.assertEqual(catalog.main(["--output", str(path)]), 1)
            self.assertEqual(path.read_text(encoding="utf-8"), "old\n")

    def test_main_leaves_output_unchanged_on_invalid_catalog(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "models.txt"
            path.write_text("old\n", encoding="utf-8")
            with (
                patch("fetch_model_catalog.resolve_key", return_value="secret-key"),
                patch(
                    "fetch_model_catalog.fetch_catalog",
                    return_value={"data": []},
                ),
                redirect_stderr(io.StringIO()),
            ):
                self.assertEqual(catalog.main(["--output", str(path)]), 1)
            self.assertEqual(path.read_text(encoding="utf-8"), "old\n")


if __name__ == "__main__":
    unittest.main()
