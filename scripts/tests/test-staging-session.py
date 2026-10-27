#!/usr/bin/env python3
"""Local tests; no network requests and no real credentials."""
import importlib.util
from email.message import Message
import io
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch
import urllib.error
import urllib.request

spec = importlib.util.spec_from_file_location("staging_session", Path(__file__).resolve().parents[1] / "staging-session.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def response(headers, url, code):
    result = urllib.response.addinfourl(io.BytesIO(b""), headers, url, code=code)
    result.msg = "Synthetic test response"
    return result


class SessionTests(unittest.TestCase):
    def test_cookie_validation_uses_exact_canonical_https_origin_and_supports_injected_credentials(self):
        seen = []
        class FakeTransport(urllib.request.BaseHandler):
            handler_order = 100
            def https_open(self, request):
                seen.append(request)
                headers = Message()
                headers.add_header("Set-Cookie", "noms_session=synthetic-test-session; Path=/; Max-Age=900; Secure; HttpOnly; SameSite=Lax")
                headers.add_header("Set-Cookie", "__cf_bm=synthetic-cloudflare-cookie; Path=/; Max-Age=1800; Secure; HttpOnly; SameSite=None")
                return response(headers, request.full_url, 204)
        build = urllib.request.build_opener
        with patch.object(module.urllib.request,"build_opener",side_effect=lambda *handlers: build(FakeTransport(),*handlers)):
            state = module.create_state("configured-proxy-placeholder")
        self.assertEqual(len(seen), 1)
        self.assertEqual(seen[0].full_url,"https://staging.nomsrecipes.com/auth/staging/session")
        self.assertEqual(seen[0].get_method(),"POST")
        self.assertEqual(state["cookies"][0]["domain"],"staging.nomsrecipes.com")
        self.assertTrue(state["cookies"][0]["httpOnly"])
        self.assertTrue(state["cookies"][0]["secure"])

    def test_redirects_are_not_followed(self):
        seen = []
        class RedirectingTransport(urllib.request.BaseHandler):
            handler_order = 100
            def https_open(self, request):
                seen.append(request.full_url)
                headers = Message()
                headers.add_header("Location", "https://nomsrecipes.com/auth/staging/session")
                return response(headers, request.full_url, 302)
        build = urllib.request.build_opener
        with patch.object(module.urllib.request,"build_opener",side_effect=lambda *handlers: build(RedirectingTransport(),*handlers)):
            with self.assertRaises(urllib.error.HTTPError): module.create_state("synthetic-test-secret")
        self.assertEqual(seen,[module.ORIGIN+"/auth/staging/session"])

    def test_unsafe_or_missing_cookies_are_rejected(self):
        cookies = [None,
                   "noms_session=synthetic; Path=/; Max-Age=900; HttpOnly; SameSite=Lax",
                   "noms_session=synthetic; Domain=.nomsrecipes.com; Path=/; Max-Age=900; Secure; HttpOnly; SameSite=Lax",
                   "noms_session=synthetic; Path=/; Max-Age=900; Secure; SameSite=Lax",
                   "noms_session=synthetic; Path=/; Max-Age=900; Secure; HttpOnly; SameSite=None",
                   "noms_session=synthetic; Path=/; Max-Age=86400; Secure; HttpOnly; SameSite=Lax"]
        build = urllib.request.build_opener
        for cookie in cookies:
            class FakeTransport(urllib.request.BaseHandler):
                handler_order = 100
                def https_open(self, request):
                    headers = Message()
                    if cookie: headers.add_header("Set-Cookie", cookie)
                    return response(headers, request.full_url, 204)
            with patch.object(module.urllib.request,"build_opener",side_effect=lambda *handlers: build(FakeTransport(),*handlers)):
                with self.assertRaises(ValueError): module.create_state("synthetic-test-secret")

    def test_repository_output_is_rejected_before_requesting_credentials(self):
        with patch("sys.argv",["staging-session.py","--output",str(Path(module.__file__).resolve().parents[1]/"auth.json")]), patch.object(module,"create_state") as create, patch("sys.stderr",new_callable=io.StringIO):
            with self.assertRaises(SystemExit): module.main()
        create.assert_not_called()

    def test_state_file_is_private_atomic_and_can_be_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/"private"/"auth.json"
            module.write_state(path,{"cookies":[],"origins":[]})
            self.assertEqual(stat.S_IMODE(path.stat().st_mode),0o600)
            os.chmod(path,0o644)
            module.write_state(path,{"cookies":[],"origins":[{"origin":module.ORIGIN,"localStorage":[]}]})
            self.assertEqual(stat.S_IMODE(path.stat().st_mode),0o600)
            self.assertEqual(json.loads(path.read_text())["origins"][0]["origin"],module.ORIGIN)
            self.assertEqual(len(list(path.parent.iterdir())),1)


if __name__ == "__main__": unittest.main()
