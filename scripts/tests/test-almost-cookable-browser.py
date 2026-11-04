#!/usr/bin/env python3
"""Almost-cookable browser/API checks on an explicitly disposable loopback app.

Requires NOMS_ENV=local, the app's SESSION_SECRET, Python Playwright, and
noms-almost-cookable-postgres labeled com.noms.almost-cookable-test=true.
Creates synthetic fixtures and removes them on completion. Cookies stay in memory.
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


def sql(statement, **values):
    command = ["docker", "exec", "-i", "noms-almost-cookable-postgres", "psql",
               "-U", "noms_ci", "-d", "noms_almost_cookable", "-Atq", "-v", "ON_ERROR_STOP=1"]
    for key, value in values.items():
        command += ["-v", key + "=" + value]
    return subprocess.run(command, input=statement, text=True, capture_output=True, check=True).stdout.strip()


def token(session):
    def encode(value):
        return base64.urlsafe_b64encode(json.dumps(value).encode()).rstrip(b"=")
    now = int(time.time())
    data = encode({"alg": "HS256", "typ": "JWT"}) + b"." + encode({"sub": session, "iat": now, "exp": now + 1800})
    signature = hmac.new(os.environ["SESSION_SECRET"].encode(), data, hashlib.sha256).digest()
    return (data + b"." + base64.urlsafe_b64encode(signature).rstrip(b"=")).decode()


async def main(args):
    from playwright.async_api import async_playwright, expect

    base = args.base_url.rstrip("/")
    assert os.environ.get("NOMS_ENV") == "local"
    assert urlsplit(base).hostname in {"127.0.0.1", "localhost"}
    label = subprocess.check_output(
        ["docker", "inspect", "noms-almost-cookable-postgres", "--format",
         '{{index .Config.Labels "com.noms.almost-cookable-test"}}'], text=True).strip()
    assert label == "true", "Explicitly labeled disposable database required"
    args.output.mkdir(parents=True, exist_ok=True)
    expect.set_options(timeout=60000)
    owner, other, session, home, cabin, foreign, root, child = [str(uuid.uuid4()) for _ in range(8)]
    checks, errors, endpoints = [], [], {}
    records, one, two, ready = [], [], [], []

    def add(title, ingredients, actor=owner, visibility="private", date="2020-01-01T00:00:00Z"):
        ident = str(uuid.uuid4())
        records.append(dict(id=ident, user_id=actor, title=title, visibility=visibility, ingredients=ingredients, updated=date))
        return ident

    ready.append(add("Ready rice", [{"name": "rice"}]))
    ready.append(add("Rice with optional garnish", [{"name": "Rice"}, {"name": "Parsley", "optional": True}]))
    ready.append(add("All optional", [{"name": "Parsley", "optional": True}]))
    for index in range(16):
        ingredients = [{"name": "Rice"}, {"name": "Milk"}]
        if index == 0:
            ingredients.append({"name": "Basil", "optional": True})
        if index == 1:
            ingredients = [{"name": "Milk", "optional": True}, {"name": "MILK"}, {"name": "rice"}]
        one.append(add(f"One ingredient away {index:02d}", ingredients))
    for index in range(15):
        two.append(add(f"Two ingredients away {index:02d}", [{"name": "Rice"}, {"name": "Milk"}, {"name": "Basil"}],
                       date="2030-01-01T00:00:00Z"))
    saved_public = add("Saved public recipe", [{"name": "Rice"}, {"name": "Milk"}], other, "public")
    saved_unlisted = add("Saved unlisted recipe", [{"name": "Rice"}, {"name": "Milk"}], other, "unlisted")
    one += [saved_public, saved_unlisted]
    hidden = add("Other private recipe", [{"name": "Rice"}, {"name": "Milk"}], other)
    add("Unsaved public recipe", [{"name": "Rice"}, {"name": "Milk"}], other, "public")
    add("Empty recipe", [])
    add("Malformed optional", [{"name": "Milk", "optional": "true"}])
    add("Malformed name", [{"name": 3}])
    for index in range(300):
        add(f"Far from cookable {index}", [{"name": name} for name in ["Milk", "Basil", "Egg", "Flour", "Butter", "Sugar"]])
    sql("""
        INSERT INTO users(id,username,display_name,email)
        VALUES(:'owner'::uuid,:'username',:'display',:'email'),
              (:'other'::uuid,:'othername',:'otherdisplay',:'otheremail');
        INSERT INTO sessions(id,user_id,expires_at) VALUES(:'session'::uuid,:'owner'::uuid,NOW()+INTERVAL '30 minutes');
        INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash)
        VALUES(:'home'::uuid,:'owner'::uuid,'Home pantry',gen_random_uuid(),repeat('0',64)),
              (:'cabin'::uuid,:'owner'::uuid,'Cabin pantry',gen_random_uuid(),repeat('0',64)),
              (:'foreign'::uuid,:'other'::uuid,'Other pantry',gen_random_uuid(),repeat('0',64));
        INSERT INTO pantry_ingredients(profile_id,name,amount,unit,quantity_zero,available,position)
        VALUES(:'home'::uuid,'Rice','0','kg',true,true,0),
              (:'home'::uuid,'Milk','2','L',false,false,1),
              (:'home'::uuid,'Basil','10','g',false,false,2),
              (:'cabin'::uuid,'Rice','1','kg',false,true,0),
              (:'cabin'::uuid,'Milk','1','L',false,true,1);
        INSERT INTO collections(id,owner_id,name,parent_collection_id)
        VALUES(:'root'::uuid,:'owner'::uuid,'Favorites',NULL),
              (:'child'::uuid,:'owner'::uuid,'Weeknight',:'root'::uuid);
        INSERT INTO recipes(id,user_id,title,visibility,ingredients,updated_at,prep_time_minutes,cook_time_minutes)
        SELECT id,user_id,title,visibility,ingredients,updated::timestamptz,10,15
        FROM jsonb_to_recordset(:'records'::jsonb)
          AS r(id uuid,user_id uuid,title text,visibility text,ingredients jsonb,updated text);
        INSERT INTO collection_recipes(collection_id,recipe_id)
        VALUES(:'root'::uuid,:'public'::uuid),(:'child'::uuid,:'public'::uuid),
              (:'child'::uuid,:'unlisted'::uuid),(:'root'::uuid,:'hidden'::uuid),
              (:'child'::uuid,:'duplicate'::uuid);
    """, owner=owner, other=other, session=session, home=home, cabin=cabin, foreign=foreign, root=root, child=child,
        username="almost_" + owner[:8], othername="almost_" + other[:8],
        display="Almost Cookable Demo " + owner[:8], otherdisplay="Other Cook " + other[:8],
        email=owner + "@example.test", otheremail=other + "@example.test",
        records=json.dumps(records), public=saved_public, unlisted=saved_unlisted, hidden=hidden, duplicate=one[0])

    def passed(message):
        checks.append(message)
        print("PASS:", message, flush=True)

    try:
        async with async_playwright() as playwright:
            browser = await playwright.chromium.launch(args=["--disable-dev-shm-usage"])
            ctx = await browser.new_context(viewport={"width": 1440, "height": 1100})
            await ctx.add_cookies([{"name": "noms_session", "value": token(session), "url": base, "httpOnly": True, "sameSite": "Lax"}])
            await ctx.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
            page = await ctx.new_page()
            page.set_default_navigation_timeout(120000)
            page.on("pageerror", lambda error: errors.append(str(error)))

            def record(request):
                for name in ["find_pantry_recipes", "get_kitchen"]:
                    if request.method == "POST" and "/api/" + name in request.url:
                        endpoints[name] = request.url
            page.on("request", record)
            await page.goto(base + "/kitchen/pantry/" + home, wait_until="networkidle")
            section = page.locator(".pantry-cookable")
            cards = section.locator(".recipe-card")
            await expect(cards).to_have_count(3)

            async def post(payload, client=ctx, ok=True):
                response = await client.request.post(endpoints["find_pantry_recipes"], data=json.dumps(payload),
                                                     headers={"Content-Type": "application/json", "Origin": base})
                if ok:
                    assert response.ok, (response.status, await response.text())
                    return await response.json()
                assert not response.ok, await response.text()

            body = {"expected_account_id": owner, "pantry_id": home, "pantry_revision": 0, "cursor": None}
            default = await post(body)
            assert default["mode"] == "cookable"
            assert {r["id"] for r in default["recipes"]} == set(ready)
            await section.get_by_role("button", name="Almost cookable", exact=True).click()
            await expect(cards).to_have_count(12)
            await expect(section.get_by_role("button", name="Almost cookable", exact=True)).to_have_attribute("aria-pressed", "true")
            await expect(section).to_contain_text("1 required ingredient missing")
            await expect(section.locator(".pantry-cookable-missing").first).to_contain_text("Milk")
            passed("Default cookable mode remains compatible; almost-cookable cards explain required shortages")

            seen, cursor, sizes = [], None, []
            started = time.perf_counter()
            first = None
            while True:
                result = await post({**body, "mode": "almost_cookable", "cursor": cursor})
                first = first or result
                assert result["mode"] == "almost_cookable"
                ids = [r["id"] for r in result["recipes"]]
                assert set(result["required_ingredients_missing"]) == set(ids)
                assert all(1 <= len(names) <= 2 for names in result["required_ingredients_missing"].values())
                assert all("ingredients" not in r and "instructions" not in r for r in result["recipes"])
                seen += ids
                sizes.append(len(ids))
                cursor = result["next_cursor"]
                if cursor is None:
                    break
                assert len(sizes) < 4
            assert seen == sorted(one, reverse=True) + sorted(two, reverse=True), seen
            assert sizes == [12, 12, 9], sizes
            (args.output / "timing.json").write_text(json.dumps({"three_pages_seconds": time.perf_counter() - started, "candidate_recipes": len(records)}, indent=2))
            passed("Global ranking and three API pages: 1 missing before 2, stable UUID ties, distinct optional/required names and bounded metadata")

            await section.get_by_role("button", name="Next recipes", exact=True).click()
            await expect(section).to_contain_text("2 required ingredients missing")
            await expect(cards).to_have_count(12)
            await section.get_by_role("button", name="Next recipes", exact=True).click()
            await expect(cards).to_have_count(9)
            await section.get_by_role("button", name="Previous recipes", exact=True).click()
            await expect(cards).to_have_count(12)
            await section.get_by_role("button", name="Can cook now", exact=True).click()
            await expect(cards).to_have_count(3)
            await section.get_by_role("button", name="Almost cookable", exact=True).click()
            await expect(cards).to_have_count(12)
            await expect(section.get_by_role("button", name="Previous recipes", exact=True)).to_be_disabled()
            passed("Client pagination preserves rank and switching readiness views resets the page")

            await post({**body, "cursor": first["next_cursor"]}, ok=False)
            await post({**body, "mode": "almost_cookable", "pantry_id": foreign}, ok=False)
            await post({**body, "mode": "almost_cookable", "pantry_revision": 1}, ok=False)
            await post({**body, "mode": "unknown"}, ok=False)
            guest = await browser.new_context()
            await post({**body, "mode": "almost_cookable"}, guest, ok=False)
            passed("API rejects cross-mode cursors, foreign pantries, stale revisions, invalid modes and anonymous reads")

            for width in [320, 390, 760, 1440]:
                await page.set_viewport_size({"width": width, "height": 844 if width < 760 else 1000})
                assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), width
                if width in [390, 1440]:
                    await section.evaluate("el => {el.style.scrollMarginTop='110px'; el.scrollIntoView({block:'start',behavior:'instant'});}")
                    await page.screenshot(path=str(args.output / f"almost-cookable-{width}.png"), full_page=False)
            await page.get_by_role("button", name="Toggle theme", exact=True).click()
            await page.screenshot(path=str(args.output / "almost-cookable-dark.png"), full_page=False)
            await page.get_by_role("button", name="Toggle theme", exact=True).click()
            passed("Missing-name cards fit mobile/desktop widths and light/dark themes")

            # A late response from the previous mode must not replace this view.
            requested, release = asyncio.Event(), asyncio.Event()
            async def held(route):
                if route.request.post_data_json.get("mode") == "cookable":
                    requested.set()
                    await release.wait()
                try:
                    await route.continue_()
                except Exception:
                    pass  # An unmounted request may be aborted.
            await page.route("**/api/find_pantry_recipes*", held)
            await section.get_by_role("button", name="Can cook now", exact=True).click()
            await asyncio.wait_for(requested.wait(), timeout=30)
            await expect(cards).to_have_count(0)
            await section.get_by_role("button", name="Almost cookable", exact=True).click()
            await expect(cards).to_have_count(12)
            release.set()
            await page.wait_for_timeout(500)
            await expect(section.get_by_role("button", name="Almost cookable", exact=True)).to_have_attribute("aria-pressed", "true")
            await expect(cards).to_have_count(12)
            await page.unroute("**/api/find_pantry_recipes*", held)
            passed("View switching hides pending cards and ignores delayed results from the prior view")

            await page.get_by_role("button", name="Restore Milk", exact=True).click()
            await expect(section.get_by_role("button", name="Almost cookable", exact=True)).to_have_attribute("aria-pressed", "true")
            await expect(cards).to_have_count(12)
            await expect(section).not_to_contain_text("One ingredient away")
            await expect(section.locator(".pantry-cookable-missing").first).to_contain_text("Basil")
            await section.get_by_role("button", name="Can cook now", exact=True).click()
            await expect(cards).to_have_count(12)
            await expect(section).to_contain_text("One ingredient away")
            passed("Restoring stock keeps the selected view, removes newly cookable recipes and promotes them to Can cook now")

            await section.get_by_role("button", name="Almost cookable", exact=True).click()
            await page.route("**/api/find_pantry_recipes*", lambda route: route.abort())
            await section.get_by_role("button", name="Next recipes", exact=True).click()
            await expect(section.get_by_role("button", name="Retry recipe check")).to_be_visible()
            await expect(cards).to_have_count(0)
            await page.unroute("**/api/find_pantry_recipes*")
            await section.get_by_role("button", name="Retry recipe check").click()
            await expect(cards).to_have_count(12)
            await expect(section.get_by_role("button", name="Almost cookable", exact=True)).to_have_attribute("aria-pressed", "true")
            await page.get_by_role("button", name="Restore Basil", exact=True).click()
            await expect(cards).to_have_count(0)
            await expect(section).to_contain_text("No almost-cookable recipes yet")
            passed("Failed recipe reads hide results; retry preserves readiness view; complete coverage shows the almost-cookable empty state")

            source = Path("src/db/pantry_recipes.rs").read_text()
            query = source.split('pub(super) const CANDIDATES: &str = "', 1)[1].split('";', 1)[0]
            query = query.replace("$1", "'" + owner + "'::uuid").replace("$2", "NULL").replace("$3", "NULL").replace("$4", str(1024 * 1024))
            (args.output / "query-plan.txt").write_text(sql("EXPLAIN (ANALYZE, BUFFERS) " + query))
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
