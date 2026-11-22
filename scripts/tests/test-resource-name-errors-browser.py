#!/usr/bin/env python3
"""Capture real name-conflict UI states on an isolated loopback Noms app.

Requires local-mode Noms, a disposable container named noms-name-error-postgres,
SESSION_SECRET for that app, and Python Playwright. No production credentials.
Cookies stay in memory and are never included in the output.
"""
import argparse
import asyncio
import base64
import hashlib
import hmac
import json
import os
import re
import subprocess
import time
import uuid
from pathlib import Path
from urllib.parse import urlsplit


def sql(statement):
    return subprocess.run(
        ["docker", "exec", "-i", "noms-name-error-postgres", "psql",
         "-U", "noms_ci", "-d", "noms_name_errors", "-At", "-v", "ON_ERROR_STOP=1"],
        input=statement, text=True, capture_output=True, check=True,
    ).stdout.strip()


def token(session):
    def encode(value):
        return base64.urlsafe_b64encode(json.dumps(value).encode()).rstrip(b"=")
    now = int(time.time())
    data = encode({"alg": "HS256", "typ": "JWT"}) + b"." + encode(
        {"sub": session, "iat": now, "exp": now + 900})
    signature = hmac.new(os.environ["SESSION_SECRET"].encode(), data, hashlib.sha256).digest()
    return (data + b"." + base64.urlsafe_b64encode(signature).rstrip(b"=")).decode()


async def main(args):
    from playwright.async_api import async_playwright, expect

    base = args.base_url.rstrip("/")
    assert os.environ.get("NOMS_ENV") == "local", "Local mode is required"
    assert urlsplit(base).hostname in {"127.0.0.1", "localhost"}, "Loopback app only"
    expect.set_options(timeout=60000)
    args.output.mkdir(parents=True, exist_ok=True)
    owner, session = str(uuid.uuid4()), str(uuid.uuid4())
    ids = {key: str(uuid.uuid4()) for key in [
        "collection", "collection_edit", "pantry", "pantry_edit", "grocery", "grocery_edit"
    ]}
    sql(f"""
        INSERT INTO users(id,username,display_name,email)
        VALUES('{owner}','demo_{owner[:8]}','Demo Cook','demo-{owner}@example.test');
        INSERT INTO sessions(id,user_id,expires_at)
        VALUES('{session}','{owner}',NOW()+INTERVAL '15 minutes');
        INSERT INTO collections(id,owner_id,name)
        VALUES('{ids["collection"]}','{owner}','Weeknight dinners');
        INSERT INTO collections(id,owner_id,parent_collection_id,name)
        VALUES('{ids["collection_edit"]}','{owner}','{ids["collection"]}','Weekend cooking');
        INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash)
        VALUES('{ids["pantry"]}','{owner}','Home pantry',gen_random_uuid(),repeat('0',64)),
              ('{ids["pantry_edit"]}','{owner}','Cabin pantry',gen_random_uuid(),repeat('0',64));
        INSERT INTO grocery_lists(id,owner_id,name,request_id,request_hash)
        VALUES('{ids["grocery"]}','{owner}','Weekly shopping',gen_random_uuid(),repeat('0',64)),
              ('{ids["grocery_edit"]}','{owner}','Weekend shopping',gen_random_uuid(),repeat('0',64));
    """)
    checks = []
    async with async_playwright() as playwright:
        browser = await playwright.chromium.launch(args=["--disable-dev-shm-usage"])
        context = await browser.new_context(viewport={"width": 1440, "height": 1100})
        await context.add_cookies([{
            "name": "noms_session", "value": token(session), "url": base,
            "httpOnly": True, "sameSite": "Lax",
        }])
        page = await context.new_page()
        page.set_default_navigation_timeout(120000)

        async def navigate(path):
            await page.goto(base + path, wait_until="networkidle")
            await expect(page.locator("h1").first).to_be_visible()

        async def capture(kind, operation, message):
            alert = page.get_by_role("alert").filter(has_text=message).last
            await expect(alert).to_have_text(message)
            await alert.scroll_into_view_if_needed()
            await page.screenshot(path=str(args.output / f"{kind}-{operation}-desktop.png"), full_page=True)
            if operation == "create":
                await page.set_viewport_size({"width": 390, "height": 844})
                await expect(alert).to_be_visible()
                await page.screenshot(path=str(args.output / f"{kind}-{operation}-mobile.png"), full_page=True)
                await page.set_viewport_size({"width": 1440, "height": 1100})
            checks.append(f"{kind} {operation}: database conflict shown in the hydrated client")

        async def count(table):
            return int(sql(f"SELECT count(*) FROM {table} WHERE owner_id='{owner}'"))

        await navigate("/collections/new")
        name = page.locator("input[name=collection_name]")
        await name.fill("weeknight  dinners")
        await page.get_by_role("button", name="Create collection", exact=True).click()
        await capture("collection", "create", "You already have a collection with this name.")
        assert await count("collections") == 2
        await expect(name).to_have_value("weeknight  dinners")
        await name.fill("Baking favorites")
        await page.get_by_role("button", name="Create collection", exact=True).click()
        await page.wait_for_url(re.compile(r".*/collections/[0-9a-f-]{36}$"))
        assert await count("collections") == 3
        checks.append("collection create: inputs retained; rename and retry succeed")

        await navigate(f'/collections/{ids["collection_edit"]}/edit')
        await page.locator("input[name=collection_name]").fill("Weeknight dinners")
        await page.get_by_role("button", name="Save changes", exact=True).click()
        await capture("collection", "rename", "You already have a collection with this name.")
        assert sql(f"SELECT name FROM collections WHERE id='{ids['collection_edit']}'") == "Weekend cooking"
        await expect(page.get_by_role("button", name="Save changes", exact=True)).to_be_enabled()

        await navigate("/kitchen/pantry/new")
        name = page.get_by_label("Pantry name", exact=True)
        await name.fill("home  pantry")
        await page.get_by_role("button", name="Create pantry", exact=True).click()
        await capture("pantry", "create", "You already have a pantry with this name.")
        assert await count("kitchen_profiles") == 2
        await expect(name).to_be_enabled()
        await name.fill("Baking pantry")
        await page.get_by_role("button", name="Create pantry", exact=True).click()
        await page.wait_for_url(re.compile(r".*/kitchen/pantry/[0-9a-f-]{36}$"))
        assert await count("kitchen_profiles") == 3
        checks.append("pantry create: inputs retained; rename and retry succeed")

        await navigate(f'/kitchen/pantry/{ids["pantry_edit"]}')
        await page.get_by_role("button", name="Edit pantry", exact=True).click()
        dialog = page.get_by_role("dialog")
        await dialog.get_by_label("Pantry name", exact=True).fill("Home pantry")
        await dialog.get_by_role("button", name="Save pantry", exact=True).click()
        await capture("pantry", "rename", "You already have a pantry with this name.")
        assert sql(f"SELECT name FROM kitchen_profiles WHERE id='{ids['pantry_edit']}'") == "Cabin pantry"

        await navigate("/kitchen/grocery-lists/new")
        name = page.get_by_label("List name", exact=True)
        await name.fill("weekly  shopping")
        await page.get_by_role("button", name="Create list", exact=True).click()
        await capture("grocery-list", "create", "You already have a grocery list with this name.")
        assert await count("grocery_lists") == 2
        await expect(name).to_be_enabled()
        await name.fill("Baking supplies")
        await page.get_by_role("button", name="Create list", exact=True).click()
        await page.wait_for_url(re.compile(r".*/kitchen/grocery-lists/[0-9a-f-]{36}$"))
        assert await count("grocery_lists") == 3
        checks.append("grocery create: inputs retained; rename and retry succeed")

        await navigate(f'/kitchen/grocery-lists/{ids["grocery_edit"]}')
        await page.get_by_role("button", name="Rename", exact=True).click()
        dialog = page.get_by_role("dialog")
        await dialog.get_by_label("List name", exact=True).fill("Weekly shopping")
        await dialog.get_by_role("button", name="Save name", exact=True).click()
        await capture("grocery-list", "rename", "You already have a grocery list with this name.")
        assert sql(f"SELECT name FROM grocery_lists WHERE id='{ids['grocery_edit']}'") == "Weekend shopping"
        await context.close()
        await browser.close()

    (args.output / "checks.json").write_text(json.dumps(checks, indent=2) + "\n")
    images = sorted(args.output.glob("*.png"))
    gallery = '<!doctype html><meta charset="utf-8"><title>Noms name errors</title>'
    gallery += '<style>body{font:16px system-ui;margin:24px}img{max-width:100%;border:1px solid #ddd}section{margin:32px 0}</style>'
    for image in images:
        gallery += f'<section><h2>{image.stem}</h2><img src="{image.name}" alt="{image.stem}"></section>'
    (args.output / "index.html").write_text(gallery)
    print(json.dumps({"checks": checks, "screenshots": [image.name for image in images]}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--output", type=Path, required=True)
    asyncio.run(main(parser.parse_args()))
