#!/usr/bin/env python3
"""Create Playwright storage state using a configured staging-only credential.

No secrets or cookies are printed. Requests never follow redirects.
"""
import argparse
import http.cookiejar
import json
import os
from pathlib import Path
import tempfile
import time
import urllib.error
import urllib.request

ORIGIN = "https://staging.nomsrecipes.com"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def create_state(secret):
    if not secret:
        raise ValueError("Configure NOMS_STAGING_LOGIN_SECRET in the execution environment.")
    request = urllib.request.Request(
        ORIGIN + "/auth/staging/session", data=b"", method="POST",
        headers={"Authorization": "Bearer " + secret, "Origin": ORIGIN},
    )
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(NoRedirect(), urllib.request.HTTPCookieProcessor(jar))
    with opener.open(request, timeout=30) as response:
        if response.status != 204:
            raise ValueError("Staging did not issue a session.")
    cookies = []
    session_name = os.environ.get("NOMS_SESSION_COOKIE_NAME") or "noms_session"
    for cookie in jar:
        # Cloudflare may add its own cookies; export only the application's session.
        if cookie.name != session_name:
            continue
        if cookie.domain != "staging.nomsrecipes.com" or cookie.domain_specified or not cookie.secure or cookie.path != "/":
            raise ValueError("Staging returned an unexpected cookie scope.")
        rest = {key.lower(): value for key, value in cookie._rest.items()}
        if "httponly" not in rest or str(rest.get("samesite", "")).lower() != "lax":
            raise ValueError("Staging returned unexpected cookie protections.")
        if cookie.expires is None or cookie.expires <= time.time() or cookie.expires > time.time() + 960:
            raise ValueError("Staging returned an unexpected session lifetime.")
        cookies.append({"name": cookie.name, "value": cookie.value, "domain": cookie.domain,
                        "path": "/", "expires": cookie.expires, "httpOnly": True,
                        "secure": True, "sameSite": "Lax"})
    if len(cookies) != 1:
        raise ValueError("Staging did not return exactly one session cookie.")
    return {"cookies": cookies, "origins": []}


def write_state(path, state):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False, encoding="utf-8") as file:
            temporary = Path(file.name)
            os.chmod(temporary, 0o600)
            json.dump(state, file)
        os.replace(temporary, path)
        temporary = None
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="Private Playwright storage-state file outside the repository")
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    if args.output.resolve().is_relative_to(repository):
        parser.error("Choose a storage-state path outside the repository.")
    try:
        state = create_state(os.environ.get("NOMS_STAGING_LOGIN_SECRET"))
        write_state(args.output, state)
    except urllib.error.HTTPError as error:
        parser.exit(1, f"Staging session request failed (HTTP {error.code}). No credentials were printed.\n")
    except (OSError, ValueError, urllib.error.URLError):
        parser.exit(1, "Could not prepare the staging session. Check the configured secret, staging flag and connectivity.\n")
    print("Staging browser session saved. Treat the storage-state file as a credential and delete it after testing.")


if __name__ == "__main__":
    main()
