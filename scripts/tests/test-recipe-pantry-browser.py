#!/usr/bin/env python3
"""Recipe/pantry integration against a ready, loopback, mock-auth dev stack.

Uses synthetic accounts and SQL fixtures only in the named development instance.
Cookies remain in memory. Requires Python Playwright, Chromium and Docker.
"""
import argparse
import asyncio
import json
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import urlsplit


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def sql(instance, statement):
    result = subprocess.run(
        ["docker", "exec", "-i", f"{instance}-postgres-1", "sh", "-c",
         'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -At'],
        input=statement, capture_output=True, text=True, check=True)
    return result.stdout.strip()


async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=20000)
    assert re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance)
    metadata = dict(line.split("=", 1) for line in
                    (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
    base = metadata["APP_URL"]
    assert metadata["STATE"] == "ready" and urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    args.output.mkdir(parents=True, exist_ok=True)
    checks, errors = [], []
    suffix = uuid.uuid4().hex[:10]
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])
        context = await browser.new_context(viewport={"width": 1440, "height": 1100})
        # Remote fonts are unavailable in restricted environments. Keep this
        # functional check local and let the app's font fallbacks render.
        await context.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
        page = await context.new_page()
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.set_default_navigation_timeout(120000)
        await page.goto(base + "/login", wait_until="networkidle")
        await page.get_by_role("link", name="Continue with Google").click()
        assert await page.locator("input[name=username]").count(), "Expected local mock OAuth form"
        await page.locator("input[name=username]").fill("cookability-" + suffix)
        await page.locator("textarea[name=claims]").fill(json.dumps({
            "email": f"cookability-{suffix}@example.test", "email_verified": True, "name": "Pantry Check Demo"}))
        await page.locator("input[type=submit]").click()
        await page.wait_for_url(base + "/**")
        owner = str(uuid.UUID((await (await context.request.get(base + "/api/user_profile")).json())["current_user_id"]))
        other, recipe, unknown_recipe, no_servings = [str(uuid.uuid4()) for _ in range(4)]
        recipe_rows = [{"name": "Rice", "amount": "1", "unit": "cup"},
                       {"name": "Rice", "amount": "1/2", "unit": "cup"},
                       {"name": "Salt", "amount": "1", "unit": "g"}]
        sql(args.instance, f"""
INSERT INTO users(id,username,display_name,email) VALUES('{other}','cook_source_{suffix}','Other account','cook-source-{suffix}@example.test');
INSERT INTO recipes(id,user_id,title,servings,ingredients,equipment,visibility) VALUES
('{recipe}','{other}','Rice supper {suffix}',2,{literal(json.dumps(recipe_rows))}::jsonb,'[{{"name":"Pot"}}]'::jsonb,'public'),
('{unknown_recipe}','{owner}','Seasoned rice {suffix}',2,'[{{"name":"Rice","amount":"to taste","unit":"cup"}}]'::jsonb,'[]'::jsonb,'private'),
('{no_servings}','{other}','Shared rice {suffix}',NULL,'[{{"name":"Rice","amount":"1","unit":"cup"}}]'::jsonb,'[]'::jsonb,'unlisted');
""")
        requests = []
        page.on("request", lambda r: requests.append(urlsplit(r.url).path))
        await page.goto(base + "/recipes/" + recipe, wait_until="networkidle")
        await expect(page.get_by_role("heading", name="Rice supper " + suffix)).to_be_visible()
        await expect(page.get_by_role("button", name="Check pantry", exact=True)).to_have_attribute("aria-expanded", "false")
        assert not any(path.startswith(("/api/get_kitchen", "/api/list_kitchens")) for path in requests)
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await expect(page.get_by_role("link", name="Create pantry", exact=True)).to_be_visible()
        checks.append("on-demand read and no-pantry creation prompt")
        print("PASS:", checks[-1], flush=True)

        profiles = {name: str(uuid.uuid4()) for name in ("Home pantry", "Cabin pantry", "Unmeasured pantry", "Different units", "Other account pantry")}
        statements = []
        for name, profile_id in profiles.items():
            actor = other if name == "Other account pantry" else owner
            statements.append(f"INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash) VALUES('{profile_id}','{actor}',{literal(name)},'{uuid.uuid4()}','{'a' * 64}');")
        for name, rows in [
            ("Home pantry", [("Rice", "1", "cup", True), ("Rice", "1", "cup", True), ("Salt", "2", "g", True)]),
            ("Cabin pantry", [("Rice", "1", "cup", True), ("Salt", "10", "g", False)]),
            ("Unmeasured pantry", [("Rice", "", "cup", True), ("Salt", "0", "g", True)]),
            ("Different units", [("Rice", "10", "cups", True), ("Salt", "2", "g", True)]),
        ]:
            for position, (ingredient, amount, unit, available) in enumerate(rows):
                statements.append(f"INSERT INTO pantry_ingredients(profile_id,name,amount,unit,available,quantity_zero,position) VALUES('{profiles[name]}',{literal(ingredient)},{literal(amount)},{literal(unit)},{str(available).lower()},{str(amount == '0').lower()},{position});")
        sql(args.instance, "\n".join(statements))
        async def reload_check():
            await page.reload(wait_until="networkidle")
            await expect(page.get_by_role("heading", name="Rice supper " + suffix)).to_be_visible()
            await page.get_by_role("button", name="Check pantry", exact=True).click()

        async def reopen_check():
            await page.get_by_role("button", name="Check pantry", exact=True).click()
            await page.get_by_role("button", name="Check pantry", exact=True).click()

        # Reload the on-demand inventory and exercise recoverable list failure.
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await page.route("**/api/list_kitchens*", lambda route: route.abort())
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await expect(page.get_by_role("button", name="Retry pantry check")).to_be_visible()
        await page.unroute("**/api/list_kitchens*")
        await page.get_by_role("button", name="Retry pantry check").click()
        select = page.get_by_label("Pantry", exact=True)
        await expect(select).to_be_visible()
        await expect(select.locator("option")).to_have_count(5)
        await expect(select).not_to_have_value("")
        assert "Other account pantry" not in await select.inner_text()
        await select.select_option(profiles["Home pantry"])
        verdict = page.locator(".recipe-pantry__verdict")
        await expect(verdict).to_have_text("All ingredients available")
        await expect(page.locator(".recipe-pantry__ingredient")).to_have_count(0)
        rice = page.locator(".recipe-pantry__ingredient").filter(has_text="Rice")
        await expect(rice).to_have_count(0)
        await expect(page.locator(".recipe-pantry")).not_to_contain_text("Needed:")
        await expect(page.locator(".recipe-pantry")).to_contain_text("Quantities aren’t checked.")
        checks.append("list retry, account isolation, duplicate stock/requirements, equipment excluded")
        print("PASS:", checks[-1], flush=True)

        await select.select_option(profiles["Cabin pantry"])
        await expect(verdict).to_have_text("Missing ingredients")
        await expect(rice).to_have_count(0)
        await expect(page.locator(".recipe-pantry__ingredient").filter(has_text="Salt")).to_have_text("Salt")
        await expect(page.locator(".recipe-pantry")).not_to_contain_text("Missing or out of stock")
        await expect(page.locator(".recipe-pantry__ingredient")).to_have_count(1)
        pane = page.locator(".recipe-pantry")
        await pane.evaluate("el => { el.style.scrollMarginTop='130px'; el.scrollIntoView({block:'start',behavior:'instant'}); }")
        await pane.screenshot(path=str(args.output / "missing-desktop.png"))
        await page.set_viewport_size({"width": 390, "height": 844})
        await pane.screenshot(path=str(args.output / "missing-phone.png"))
        await page.set_viewport_size({"width": 1440, "height": 1100})
        await select.select_option(profiles["Unmeasured pantry"])
        await expect(verdict).to_have_text("All ingredients available")
        await expect(rice).to_have_count(0)
        await expect(page.locator(".recipe-pantry__ingredient")).to_have_count(0)
        await select.select_option(profiles["Different units"])
        await expect(verdict).to_have_text("All ingredients available")
        await expect(rice).to_have_count(0)
        sql(args.instance, f"UPDATE pantry_ingredients SET name='rice' WHERE profile_id='{profiles['Different units']}' AND name='Rice';")
        await reload_check()
        await expect(verdict).to_have_text("All ingredients available")
        await expect(rice).to_have_count(0)
        sql(args.instance, f"UPDATE pantry_ingredients SET name='Brown rice' WHERE profile_id='{profiles['Different units']}' AND name='rice';")
        await reload_check()
        await expect(verdict).to_have_text("Missing ingredients")
        await expect(rice).to_have_text("Rice")
        checks.append("explicit out-of-stock, amounts and units ignored, case-insensitive names without aliases")
        print("PASS:", checks[-1], flush=True)

        # Never retain a green result if an inventory read fails.
        await select.select_option(profiles["Home pantry"])
        await expect(verdict).to_have_text("All ingredients available")
        started, release = asyncio.Event(), asyncio.Event()
        async def hold_inventory(route):
            started.set()
            await release.wait()
            await route.continue_()
        await page.route("**/api/get_kitchen*", hold_inventory)
        await reopen_check()
        await asyncio.wait_for(started.wait(), timeout=20)
        try:
            await expect(verdict).to_have_count(0)
            await expect(page.locator(".recipe-pantry").get_by_role("status")).to_have_text("Checking pantry…")
        finally:
            release.set()
        await expect(verdict).to_have_text("All ingredients available")
        await page.unroute("**/api/get_kitchen*", hold_inventory)
        await page.route("**/api/get_kitchen*", lambda route: route.abort())
        await reopen_check()
        await expect(page.get_by_role("button", name="Retry pantry check")).to_be_visible()
        await expect(verdict).to_have_count(0)
        await page.unroute("**/api/get_kitchen*")
        await page.get_by_role("button", name="Retry pantry check").click()
        await expect(verdict).to_have_text("All ingredients available")
        sql(args.instance, f"UPDATE pantry_ingredients SET amount='0',quantity_zero=TRUE WHERE profile_id='{profiles['Home pantry']}' AND name='Rice';")
        await reload_check()
        await expect(verdict).to_have_text("All ingredients available")
        sql(args.instance, f"UPDATE pantry_ingredients SET available=FALSE WHERE profile_id='{profiles['Home pantry']}' AND name='Rice';")
        await reload_check()
        await expect(verdict).to_have_text("Missing ingredients")
        await expect(page.locator(".recipe-pantry__ingredient")).to_have_count(1)
        await expect(rice).to_have_text("Rice")
        sql(args.instance, f"UPDATE pantry_ingredients SET available=TRUE WHERE profile_id='{profiles['Home pantry']}' AND name='Rice';")
        await reload_check()
        await expect(verdict).to_have_text("All ingredients available")
        checks.append("failed-read recovery clears old verdict; page reload reads saved changes")
        print("PASS:", checks[-1], flush=True)

        await page.get_by_role("button", name=re.compile("Scale Recipe")).click()
        await page.get_by_role("button", name="2x", exact=True).click()
        await expect(verdict).to_have_text("All ingredients available")
        await expect(page.locator(".recipe-pantry")).not_to_contain_text("Needed:")
        await page.get_by_role("button", name="1x", exact=True).click()
        await expect(verdict).to_have_text("All ingredients available")
        checks.append("desktop scaling leaves ingredient availability unchanged")
        print("PASS:", checks[-1], flush=True)
        async def screenshot_check(name):
            pane = page.locator(".recipe-pantry")
            # Keep the fixed navigation above the component in element captures.
            await pane.evaluate("el => { el.scrollIntoView({block: 'start'}); window.scrollBy(0, -120); }")
            await pane.screenshot(path=str(args.output / name))
        await screenshot_check("check-desktop.png")
        await page.screenshot(path=str(args.output / "recipe-desktop.png"), full_page=True)

        await page.set_viewport_size({"width": 390, "height": 844})
        await page.get_by_role("button", name="Increase servings", exact=True).click()
        await expect(page.locator(".mobile-servings-stepper strong")).to_have_text("3")
        await expect(verdict).to_have_text("All ingredients available")
        await page.get_by_role("button", name="Decrease servings", exact=True).click()
        await expect(verdict).to_have_text("All ingredients available")
        await screenshot_check("check-phone.png")
        for width in (320, 390, 760, 1100, 1440):
            await page.set_viewport_size({"width": width, "height": 1000})
            assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), width
        await page.get_by_role("button", name="Toggle theme", exact=True).click()
        await screenshot_check("check-dark.png")
        checks.append("mobile servings leave availability unchanged; five widths and dark theme")
        print("PASS:", checks[-1], flush=True)

        # Remembered selection, private recipe and authored textual quantity.
        await page.goto(base + "/recipes/" + unknown_recipe, wait_until="networkidle")
        await expect(page.get_by_role("heading", name="Seasoned rice " + suffix)).to_be_visible()
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await expect(page.get_by_label("Pantry", exact=True)).to_have_value(profiles["Home pantry"])
        await expect(verdict).to_have_text("All ingredients available")
        await expect(page.locator(".recipe-pantry__ingredient")).to_have_count(0)
        await page.goto(base + "/recipes/" + no_servings, wait_until="networkidle")
        await expect(page.get_by_role("heading", name="Shared rice " + suffix)).to_be_visible()
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await expect(verdict).to_have_text("All ingredients available")
        await expect(page.locator(".recipe-pantry")).to_contain_text("Quantities aren’t checked.")
        checks.append("remembered pantry; private/unlisted recipes; recipe quantities and servings ignored")
        print("PASS:", checks[-1], flush=True)

        anonymous = await browser.new_context()
        await anonymous.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
        anon = await anonymous.new_page()
        anon.on("pageerror", lambda e: errors.append(str(e)))
        await anon.goto(base + "/recipes/" + recipe, wait_until="networkidle")
        await expect(anon.locator(".recipe-pantry__hint")).to_contain_text("Sign in")
        await expect(anon.get_by_role("button", name="Check pantry", exact=True)).to_have_count(0)
        await anonymous.close()
        checks.append("public recipe stays accessible anonymously without pantry data")
        print("PASS:", checks[-1], flush=True)
        assert not errors, errors
        (args.output / "result.json").write_text(json.dumps({"checks": checks, "passed": len(checks), "page_errors": errors}, indent=2))
        print(json.dumps({"passed": len(checks), "page_errors": errors}))
        await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", default="noms-groceries")
    parser.add_argument("--output", type=Path, default=Path("/workspace/noms-recipe-pantry-e2e"))
    asyncio.run(main(parser.parse_args()))
