#!/usr/bin/env python3
"""Public-search browser/API regression against an isolated loopback Noms app.

Requires NOMS_ENV=local, SESSION_SECRET, Python Playwright and a disposable
container named noms-public-search-postgres labeled com.noms.public-search-test=true.
Uses synthetic sessions; never prints cookies or changes production data.
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
from urllib.parse import parse_qs, urlsplit


def sql(statement, **values):
    command = ["docker", "exec", "-i", "noms-public-search-postgres", "psql",
               "-U", "noms_ci", "-d", "noms_public_search", "-Atq", "-v", "ON_ERROR_STOP=1"]
    for key, value in values.items():
        command += ["-v", key + "=" + value]
    return subprocess.run(command, input=statement, text=True, capture_output=True, check=True).stdout.strip()


def token(session, expired=False):
    def encode(value):
        return base64.urlsafe_b64encode(json.dumps(value).encode()).rstrip(b"=")
    now = int(time.time())
    data = encode({"alg": "HS256", "typ": "JWT"}) + b"." + encode(
        {"sub": session, "iat": now - 100, "exp": now - 60 if expired else now + 1800})
    signature = hmac.new(os.environ["SESSION_SECRET"].encode(), data, hashlib.sha256).digest()
    return (data + b"." + base64.urlsafe_b64encode(signature).rstrip(b"=")).decode()


async def main(args):
    from playwright.async_api import async_playwright, expect

    base = args.base_url.rstrip("/")
    assert os.environ.get("NOMS_ENV") == "local", "Local mode required"
    assert urlsplit(base).hostname in {"localhost", "127.0.0.1"}, "Loopback app only"
    label = subprocess.check_output(
        ["docker", "inspect", "noms-public-search-postgres", "--format",
         '{{index .Config.Labels "com.noms.public-search-test"}}'], text=True).strip()
    assert label == "true", "An explicitly labeled disposable database is required"
    expect.set_options(timeout=60000)
    args.output.mkdir(parents=True, exist_ok=True)
    owner, other, session, revoked = [str(uuid.uuid4()) for _ in range(4)]
    needle = "publicprobe" + uuid.uuid4().hex[:10]
    checks, errors, endpoints = [], [], {}
    public_ids, owned_ids, other_hidden = set(), set(), set()
    recipes = []
    for actor, prefix in [(owner, "Owner"), (other, "Other")]:
        for visibility in ["public", "private", "unlisted"]:
            # Enough public recipes to exercise real UI pagination.
            for index in range(12 if visibility == "public" else 1):
                ident = str(uuid.uuid4())
                title = f"{needle} {prefix} {visibility} dish {index:02d}"
                recipes.append(dict(id=ident, user_id=actor, title=title, visibility=visibility,
                                    servings=2 if visibility == "public" else 99))
                if visibility == "public":
                    public_ids.add(ident)
                if actor == owner or visibility == "public":
                    owned_ids.add(ident)
                if actor == other and visibility != "public":
                    other_hidden.add(ident)
    root, child, grandchild, middle, leaf, other_public, other_private = [str(uuid.uuid4()) for _ in range(7)]
    collections = [
        dict(id=root, owner_id=owner, parent=None, name=f"{needle} Hidden root", visibility="private"),
        dict(id=child, owner_id=owner, parent=root, name=f"{needle} Public child", visibility="public"),
        dict(id=grandchild, owner_id=owner, parent=child, name=f"{needle} Public grandchild", visibility="public"),
        dict(id=middle, owner_id=owner, parent=child, name=f"{needle} Hidden middle", visibility="unlisted"),
        dict(id=leaf, owner_id=owner, parent=middle, name=f"{needle} Public leaf", visibility="public"),
        dict(id=other_public, owner_id=other, parent=None, name=f"{needle} Other public", visibility="public"),
        dict(id=other_private, owner_id=other, parent=None, name=f"{needle} Other hidden", visibility="private"),
    ]
    public_collections = {child, grandchild, leaf, other_public}

    def passed(message):
        checks.append(message)
        print("PASS:", message, flush=True)

    sql("""
        INSERT INTO users(id,username,display_name,email,bio)
        VALUES(:'owner'::uuid,:'username',:'display',:'email',:'needle'),
              (:'other'::uuid,:'othername',:'otherdisplay',:'otheremail',:'needle');
        INSERT INTO sessions(id,user_id,expires_at,revoked)
        VALUES(:'session'::uuid,:'owner'::uuid,NOW()+INTERVAL '30 minutes',false),
              (:'revoked'::uuid,:'owner'::uuid,NOW()+INTERVAL '30 minutes',true);
        INSERT INTO recipes(id,user_id,title,visibility,servings,prep_time_minutes)
        SELECT id,user_id,title,visibility,servings,CASE WHEN visibility='public' THEN 20 ELSE 900 END
        FROM jsonb_to_recordset(:'recipes'::jsonb)
            AS r(id uuid,user_id uuid,title text,visibility text,servings integer);
        INSERT INTO recipe_tags(recipe_id,tag)
        SELECT id,CASE WHEN visibility='public' THEN 'safe' ELSE 'secret' END
        FROM recipes WHERE user_id IN (:'owner'::uuid,:'other'::uuid);
        INSERT INTO collections(id,owner_id,parent_collection_id,name,visibility)
        SELECT id,owner_id,parent,name,visibility FROM jsonb_to_recordset(:'collections'::jsonb)
            AS c(id uuid,owner_id uuid,parent uuid,name text,visibility text);
        INSERT INTO collection_recipes(collection_id,recipe_id)
        SELECT :'child'::uuid,id FROM recipes WHERE user_id=:'owner'::uuid AND visibility='private';
    """, owner=owner, other=other, session=session, revoked=revoked, needle=needle,
        username="probe_" + owner[:8], othername="probe_" + other[:8],
        display=needle + " Owner Cook", otherdisplay=needle + " Other Cook",
        email="secret-" + owner + "@example.test", otheremail="secret-" + other + "@example.test",
        recipes=json.dumps(recipes), collections=json.dumps(collections), child=child)

    try:
        async with async_playwright() as playwright:
            browser = await playwright.chromium.launch(args=["--disable-dev-shm-usage"])
            guest = await browser.new_context(viewport={"width": 1440, "height": 1000})
            await guest.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
            page = await guest.new_page()
            page.set_default_navigation_timeout(120000)
            page.on("pageerror", lambda error: errors.append(str(error)))

            def remember(request):
                for name in ["search_all", "search_recipes", "search_collections", "search_users",
                             "autocomplete_entities", "suggest_recipe_titles"]:
                    if request.method == "POST" and "/api/" + name in request.url:
                        endpoints[name] = request.url
            page.on("request", remember)

            async def navigate(path):
                await page.goto(base + path, wait_until="networkidle")
                assert urlsplit(page.url).path == "/search", page.url
                await expect(page.get_by_role("heading", name="Search Noms", exact=True)).to_be_visible()

            async def api(name, payload, client=guest, headers=None, ok=True):
                response = await client.request.post(
                    endpoints[name], data=json.dumps(payload),
                    headers={"Content-Type": "application/json", "Origin": base, **(headers or {})})
                if ok:
                    assert response.ok, (name, response.status, await response.text())
                    return await response.json()
                return response

            await navigate("/search")
            field = page.locator("#search-query")
            await field.fill(needle)
            await expect(page.get_by_role("option").first).to_be_visible()
            await field.press("Escape")
            await field.press("Enter")
            await expect(page.locator(".search-results")).to_contain_text("24 found")
            grouped = await api("search_all", {"request": {"text": needle}})
            assert (grouped["recipe_total_count"], grouped["collection_total_count"], grouped["user_total_count"]) == (24, 4, 2)
            serialized = json.dumps(grouped)
            assert "Hidden root" not in serialized and "Hidden middle" not in serialized and "secret-" not in serialized
            passed("Guest: direct search and All tab return only public content and public profile fields")

            for tab, name, expected_count in [
                ("Recipes", "search_recipes", 24), ("Collections", "search_collections", 4), ("Cooks", "search_users", 2),
            ]:
                await page.get_by_role("navigation", name="Search type").get_by_role("button", name=tab, exact=True).click()
                await expect(page.locator(".results-heading")).to_contain_text(str(expected_count) + " found")
                response = await api(name, {"request": {"text": needle}})
                assert response["total_count"] == expected_count
                if tab == "Recipes":
                    assert {row["recipe"]["id"] for row in response["results"]} <= public_ids
                    assert all(row["visibility"] == "public" or row["count"] == 0 for row in response["facets"]["visibility"])
                    assert all(row["tag"] != "secret" for row in response["facets"]["tags"])
                    before = await page.locator(".search-recipe-grid .recipe-card").count()
                    assert before == 20
                    await page.get_by_role("button", name="Next", exact=True).click()
                    await expect(page.locator(".search-recipe-grid .recipe-card")).to_have_count(4)
                    assert parse_qs(urlsplit(page.url).query)["page"] == ["2"]
                    filtered = await api(name, {"request": {"text": needle, "tags_all": ["secret"]}})
                    assert filtered["total_count"] == 0
                    mine = await api(name, {"request": {"text": needle, "visibility": "mine_only"}})
                    assert mine["total_count"] == 0
                else:
                    await field.fill(needle + (" Public" if tab == "Collections" else " Owner"))
                    await expect(page.get_by_role("option").first).to_be_visible()
                    suggestions = await api("autocomplete_entities", {"scope": "collections" if tab == "Collections" else "users", "text": needle})
                    if tab == "Collections":
                        assert {row["id"] for row in suggestions} <= public_collections
                        paths = {row["collection"]["id"]: row["breadcrumb"] for row in response["results"]}
                        assert paths[child] == [needle + " Public child"]
                        assert paths[grandchild] == [needle + " Public child", needle + " Public grandchild"]
                        assert paths[leaf] == [needle + " Public leaf"]
                    assert "secret-" not in json.dumps(suggestions)
                    await field.press("Escape")
                    await field.fill(needle)
                passed("Guest: " + tab + " results, counts and suggestions respect visibility")

            await navigate("/search?q=" + needle + "nomatches")
            await expect(page.locator(".search-results")).to_contain_text("No results found")
            await page.goto(base + "/", wait_until="networkidle")
            landing = page.locator("#landing-recipe-query")
            await landing.fill(needle)
            await expect(page.get_by_role("option").first).to_be_visible()
            await landing.press("Escape")
            await page.get_by_role("search", name="Find a recipe").get_by_role("button", name="Search", exact=True).click()
            await expect(page.locator(".search-results")).to_contain_text("24 found")
            assert urlsplit(page.url).path == "/search"
            navbar = page.get_by_role("search", name="Site search", exact=True)
            await navbar.get_by_role("combobox").fill(needle)
            await navbar.get_by_role("combobox").press("Escape")
            await navbar.get_by_role("button", name="Submit search").click()
            await expect(page.locator(".search-results")).to_contain_text("24 found")
            await page.screenshot(path=str(args.output / "public-search-desktop.png"), full_page=True)
            await page.set_viewport_size({"width": 390, "height": 844})
            await expect(field).to_be_visible()
            await page.screenshot(path=str(args.output / "public-search-mobile.png"), full_page=True)
            passed("Guest: landing and navbar submit to public Search; empty state and mobile rendering work")

            signed = await browser.new_context()
            async def cookie(client, value):
                await client.add_cookies([{"name": "noms_session", "value": value, "url": base,
                                          "httpOnly": True, "sameSite": "Lax"}])
            await cookie(signed, token(session))
            owner_response = await api("search_recipes", {"request": {"text": needle, "page_size": 100}}, signed)
            assert owner_response["total_count"] == 26
            assert {row["recipe"]["id"] for row in owner_response["results"]} == owned_ids
            owned_collections = await api("search_collections", {"request": {"text": needle}}, signed)
            assert owned_collections["total_count"] == 6
            assert other_private not in {row["collection"]["id"] for row in owned_collections["results"]}
            private_title = next(row["title"] for row in recipes if row["user_id"] == owner and row["visibility"] == "private")
            assert await api("suggest_recipe_titles", {"text": private_title, "public_only": False}, signed) == [private_title]
            assert await api("suggest_recipe_titles", {"text": private_title, "public_only": False}) == []
            for invalid in ["invalid-token", token(session, expired=True), token(revoked)]:
                await cookie(signed, invalid)
                response = await api("search_recipes", {"request": {"text": needle, "page_size": 100}}, signed)
                assert {row["recipe"]["id"] for row in response["results"]} == public_ids
            passed("Sessions: own private/unlisted content remains searchable; missing/invalid/expired/revoked sessions see public only")

            await cookie(signed, token(session))
            owner_page = await signed.new_page()
            await owner_page.goto(base + "/search?q=" + needle + "&scope=recipes", wait_until="networkidle")
            await expect(owner_page.locator(".results-heading")).to_contain_text("26 found")
            await owner_page.get_by_role("button", name="Account menu", exact=True).click()
            await owner_page.locator(".navbar-logout-form").first.locator("button").click()
            await owner_page.wait_for_url(base + "/")
            await owner_page.goto(base + "/search?q=" + needle + "&scope=recipes", wait_until="networkidle")
            await expect(owner_page.locator(".results-heading")).to_contain_text("24 found")
            assert private_title not in await owner_page.locator(".search-results").inner_text()
            passed("Client: logout and returning to Search remove owner-only results")

            # Exhaust only after functional checks. Spoofed forwarding/internal
            # headers must not create a fresh anonymous actor bucket.
            exhausted = False
            for _ in range(610):
                response = await api("search_users", {"request": {"text": needle + "nomatches"}}, ok=False)
                if not response.ok:
                    assert "Too many" in await response.text() or "rate" in (await response.text()).lower()
                    exhausted = True
                    break
            assert exhausted, "Anonymous search budget was not enforced"
            for index in range(3):
                response = await api("search_users", {"request": {"text": needle}},
                                     headers={"X-Forwarded-For": "198.51.100." + str(index + 1),
                                              "X-Noms-Resolved-Client-Ip": "203.0.113." + str(index + 1)}, ok=False)
                assert not response.ok, "A forged identity bypassed the anonymous request budget"
            # Fresh authenticated session has a separate actor budget.
            fresh_session = str(uuid.uuid4())
            sql("INSERT INTO sessions(id,user_id,expires_at) VALUES(:'session'::uuid,:'owner'::uuid,NOW()+INTERVAL '15 minutes');",
                session=fresh_session, owner=owner)
            await cookie(signed, token(fresh_session))
            assert (await api("search_users", {"request": {"text": needle}}, signed))["total_count"] == 2
            passed("Limits: anonymous budget is enforced, forged headers cannot reset it, and verified-user budget remains separate")
            assert not errors, errors
            await browser.close()
    finally:
        sql("DELETE FROM users WHERE id IN (:'owner'::uuid,:'other'::uuid);", owner=owner, other=other)
        (args.output / "checks.json").write_text(json.dumps({"checks": checks, "page_errors": errors}, indent=2) + "\n")

    print(json.dumps({"checks": len(checks), "page_errors": errors}, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--output", type=Path, required=True)
    asyncio.run(main(parser.parse_args()))
