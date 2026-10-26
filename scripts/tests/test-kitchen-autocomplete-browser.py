#!/usr/bin/env python3
"""Ingredient autocomplete in Kitchen against a ready local mock-auth stack.

Uses synthetic accounts and a private recipe fixture only in the named instance.
Requires Chromium, Python Playwright and Docker. Cookies remain in memory.
"""
import argparse
import asyncio
import json
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import urlsplit


async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=20000)
    assert re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance)
    metadata = dict(line.split("=", 1) for line in
                    (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
    base = metadata["APP_URL"]
    assert metadata["STATE"] == "ready" and urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    args.output.mkdir(parents=True, exist_ok=True)
    checks, errors, lookups, catalogs = [], [], [], []
    suffix = uuid.uuid4().hex[:10]

    def passed(text):
        checks.append(text)
        print("PASS:", text, flush=True)

    async with async_playwright() as playwright:
        browser = await playwright.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])

        async def new_context(**kwargs):
            context = await browser.new_context(**kwargs)
            await context.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
            context.on("page", lambda page: page.on("pageerror", lambda error: errors.append(str(error))))
            return context

        context = await new_context(viewport={"width": 1440, "height": 1100}, has_touch=True)
        page = await context.new_page()
        page.set_default_navigation_timeout(120000)
        await page.goto(base + "/login", wait_until="networkidle")
        await page.get_by_role("link", name="Continue with Google").click()
        assert await page.locator("input[name=username]").count(), "Expected local mock OAuth form"
        await page.locator("input[name=username]").fill("kitchen-suggestions-" + suffix)
        await page.locator("textarea[name=claims]").fill(json.dumps({
            "email": f"kitchen-suggestions-{suffix}@example.test", "email_verified": True, "name": "Kitchen Suggestions Demo"}))
        await page.locator("input[type=submit]").click()
        await page.wait_for_url(base + "/**")
        owner = str(uuid.UUID((await (await context.request.get(base + "/api/user_profile")).json())["current_user_id"]))
        manifest_response = await context.request.get(base + "/api/vocabulary/ingredients/version.json")
        assert manifest_response.ok
        manifest = await manifest_response.json()
        catalog_response = await context.request.get(base + manifest["url"])
        assert catalog_response.ok
        entries = (await catalog_response.json())["entries"]
        ingredients = {name: next(entry for entry in entries if entry["label"].casefold() == name)
                       for name in ["rice", "sugar", "flour", "salt"]}
        rice, sugar, flour, salt = [ingredients[name]["label"] for name in ["rice", "sugar", "flour", "salt"]]
        page.on("request", lambda request: lookups.append(request.url) if "/api/ingredient_suggestions" in request.url else None)
        page.on("request", lambda request: catalogs.append(request.url) if re.search(r"/api/vocabulary/ingredients/[a-f0-9]{64}\.json", request.url) else None)

        async def choose(field, name, method="keyboard", target=page):
            await field.fill(name)
            option = target.get_by_role("option", name=name, exact=True).first
            await expect(option).to_be_visible()
            assert await option.inner_text() == name, "Options should contain only the ingredient label"
            controls = await field.get_attribute("aria-controls")
            assert controls and await target.locator(f'[id="{controls}"]').count() == 1
            if method == "keyboard":
                rows = await target.locator(f'[id="{controls}"]').get_by_role("option").all_text_contents()
                for _ in range(rows.index(name) + 1):
                    await field.press("ArrowDown")
                await expect(option).to_have_attribute("aria-selected", "true")
                active = await field.get_attribute("aria-activedescendant")
                assert active and await target.locator(f'[id="{active}"]').inner_text() == name
                await field.press("Enter")
            elif method == "touch":
                await option.tap()
            else:
                await option.click()
            await expect(field).to_have_value(name)
            await expect(field).to_have_attribute("aria-expanded", "false")

        await page.goto(base + "/kitchen/pantry/new", wait_until="networkidle")
        await page.wait_for_function("""async () => Boolean(await (await caches.open(
            'noms-ingredient-vocabulary-v1')).match('/__noms_cache/ingredient-vocabulary-v1'))""")
        await page.get_by_label("Pantry name", exact=True).fill("Autocomplete pantry " + suffix)
        add = page.locator("form.pantry-add")
        field = add.get_by_role("combobox", name="Ingredient name", exact=True)
        await expect(field).to_have_attribute("maxlength", "500")
        assert await field.evaluate("el => el.required")
        await add.get_by_label("Amount", exact=True).fill("2")
        await add.get_by_label("Unit", exact=True).fill("cup")
        await choose(field, rice)
        await expect(page.locator(".pantry-draft-row")).to_have_count(0)
        await expect(add.get_by_label("Amount", exact=True)).to_have_value("2")
        await expect(add.get_by_label("Unit", exact=True)).to_have_value("cup")
        await add.get_by_role("button", name="Add ingredient", exact=True).click()
        await page.get_by_role("button", name="Edit initial " + rice, exact=True).click()
        dialog = page.get_by_role("dialog")
        edited = dialog.get_by_role("combobox", name="Ingredient name", exact=True)
        assert await edited.get_attribute("aria-controls") != await field.get_attribute("aria-controls")
        await edited.fill(sugar)
        await expect(page.get_by_role("listbox")).to_be_visible()
        await edited.press("Escape")
        await expect(dialog).to_be_visible()
        await expect(page.get_by_role("listbox")).to_have_count(0)
        await choose(edited, sugar)
        await expect(dialog).to_be_visible()
        await dialog.get_by_role("button", name="Save item", exact=True).click()
        await expect(page.locator(".pantry-draft-row")).to_contain_text(sugar)
        await choose(field, rice)
        await page.get_by_role("button", name="Create pantry", exact=True).click()
        await page.wait_for_url(re.compile(r".*/kitchen/pantry/[0-9a-f-]{36}$"))
        pantry_url = page.url
        await expect(page.locator(".pantry-row")).to_have_count(2)
        await expect(page.locator(".pantry-row").filter(has_text=sugar)).to_contain_text("2 cup")
        passed("pantry create/draft editing: keyboard selection, distinct IDs, Escape keeps modal open, unsubmitted entry retained")

        add = page.locator("form.pantry-add")
        await add.get_by_label("Amount", exact=True).fill("1")
        await add.get_by_label("Unit", exact=True).fill("kg")
        await choose(add.get_by_role("combobox", name="Ingredient name", exact=True), flour, method="mouse")
        await add.get_by_role("button", name="Add ingredient", exact=True).click()
        await expect(page.locator(".pantry-row").filter(has_text=flour)).to_contain_text("1 kg")
        await page.get_by_role("button", name="Mark " + flour + " out", exact=True).click()
        await page.get_by_role("button", name="Edit " + flour, exact=True).click()
        editor = page.locator(".pantry-editor")
        await choose(editor.get_by_role("combobox", name="Ingredient name", exact=True), salt)
        await editor.get_by_role("button", name="Save changes", exact=True).click()
        await expect(page.get_by_role("button", name="Restore " + salt, exact=True)).to_be_visible()
        await page.reload(wait_until="networkidle")
        await expect(page.locator(".pantry-row").filter(has_text=salt)).to_contain_text("1 kg")
        passed("pantry quick add and saved edits preserve quantities, units and out-of-stock state across reload")

        await page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
        await page.get_by_label("List name", exact=True).fill("Autocomplete groceries " + suffix)
        add = page.locator("form.grocery-add-row")
        await add.get_by_label("Amount", exact=True).fill("3")
        await add.get_by_label("Unit", exact=True).fill("bag")
        await choose(add.get_by_role("combobox", name="Item name", exact=True), rice)
        await expect(page.locator(".grocery-draft-row")).to_have_count(0)
        await add.get_by_role("button", name="Add", exact=True).click()
        await add.get_by_role("combobox", name="Item name", exact=True).fill("My household soap " + suffix)
        await page.get_by_role("button", name="Create list", exact=True).click()
        await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        grocery_url = page.url
        await expect(page.locator(".grocery-row")).to_have_count(2)
        await expect(page.locator(".grocery-row").filter(has_text=rice)).to_contain_text("3 bag")
        passed("grocery create: suggestions and arbitrary household names coexist; amounts/units retained")

        await page.set_viewport_size({"width": 390, "height": 844})
        add = page.locator("form.grocery-add-row")
        await add.get_by_label("Amount", exact=True).fill("4")
        await add.get_by_label("Unit", exact=True).fill("jar")
        await choose(add.get_by_role("combobox", name="Item name", exact=True), sugar, method="touch")
        await add.get_by_role("button", name="Add", exact=True).click()
        row = page.locator(".grocery-row").filter(has_text=sugar)
        await expect(row).to_contain_text("4 jar")
        await row.locator('input[type="checkbox"]').check()
        await row.get_by_role("button", name="Edit " + sugar, exact=True).click()
        dialog = page.get_by_role("dialog")
        edited = dialog.get_by_role("combobox", name="Ingredient / item name", exact=True)
        await choose(edited, salt)
        started, release = asyncio.Event(), asyncio.Event()

        async def hold_save(route):
            started.set()
            await release.wait()
            await route.continue_()

        await page.route("**/api/update_grocery_item*", hold_save)
        await dialog.get_by_role("button", name="Save item", exact=True).click()
        await asyncio.wait_for(started.wait(), timeout=20)
        try:
            await expect(edited).to_be_disabled()
            await expect(dialog.get_by_role("listbox")).to_have_count(0)
        finally:
            release.set()
        await expect(dialog).to_have_count(0)
        await page.unroute("**/api/update_grocery_item*", hold_save)
        await page.reload(wait_until="networkidle")
        row = page.locator(".grocery-row").filter(has_text=salt)
        await expect(row).to_contain_text("4 jar")
        await expect(row.locator('input[type="checkbox"]')).to_be_checked()
        passed("grocery quick add supports touch; edit modal disables during saves and preserves checked status")

        await page.set_viewport_size({"width": 1440, "height": 1100})
        recipe_id = str(uuid.uuid4())
        title = "Kitchen autocomplete source " + suffix
        subprocess.run(["docker", "exec", "-i", f"{args.instance}-postgres-1", "sh", "-c",
                        'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -At'],
                       input=f"INSERT INTO recipes(id,user_id,title,servings,ingredients,visibility) VALUES('{recipe_id}','{owner}','{title}',2,'[{{\"name\":\"Authored custom rice\",\"amount\":\"2\",\"unit\":\"cup\"}}]'::jsonb,'private');",
                       text=True, capture_output=True, check=True)
        await page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
        await page.get_by_label("List name", exact=True).fill("Recipe groceries " + suffix)
        await page.get_by_role("button", name="From recipes").click()
        await page.get_by_role("searchbox", name="Search accessible recipes").fill(suffix)
        await page.locator(".grocery-recipe-search").first.get_by_role("button", name="Search", exact=True).click()
        await page.locator(".grocery-recipe-option").filter(has_text=title).get_by_role("button", name="Add", exact=True).click()
        await expect(page.locator(".grocery-selected-recipe")).to_contain_text(title)
        await page.get_by_role("button", name="Create list", exact=True).click()
        await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        row = page.locator(".grocery-row").filter(has_text="Authored custom rice")
        await expect(row).to_contain_text("2 cup")
        await row.get_by_role("button", name="Recipe details for Authored custom rice", exact=True).click()
        await row.get_by_role("button", name="Edit Authored custom rice", exact=True).click()
        dialog = page.get_by_role("dialog")
        await choose(dialog.get_by_role("combobox", name="Ingredient / item name", exact=True), rice)
        await dialog.get_by_role("button", name="Save item", exact=True).click()
        await expect(page.locator(".grocery-inspector")).to_contain_text("Authored custom rice: 2 cup")
        await expect(page.locator(".grocery-inspector")).to_contain_text("Edited on this list")
        passed("recipe-generated names stay authored until edited; suggestion edits preserve original contributions")

        for path, selector, name, artifact in [
            ("/kitchen/pantry/new", "form.pantry-add", "Ingredient name", "pantry"),
            (grocery_url.removeprefix(base), "form.grocery-add-row", "Item name", "grocery"),
        ]:
            await page.goto(base + path, wait_until="networkidle")
            field = page.locator(selector).get_by_role("combobox", name=name, exact=True)
            for width in [320, 390, 760, 1100, 1440]:
                await page.set_viewport_size({"width": width, "height": 1000})
                await field.fill("ri")
                menu = page.get_by_role("listbox")
                await expect(menu).to_be_visible()
                assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), (artifact, width)
                box = await menu.bounding_box()
                assert box and box["x"] >= 0 and box["x"] + box["width"] <= width + 1, (artifact, width, box)
                if width == 320:
                    assert box["width"] >= 200, "Quick-add suggestions should extend beyond the compact name column"
                assert await menu.get_by_role("option").first.evaluate("el => el.getBoundingClientRect().height >= 44")
                if width in [390, 1440]:
                    await field.evaluate("el => el.scrollIntoView({block:'center',behavior:'instant'})")
                    await page.screenshot(path=str(args.output / f"{artifact}-{width}.png"), full_page=True)
                await field.press("Escape")
            await page.get_by_role("button", name="Toggle theme", exact=True).click()
            await field.fill("ri")
            await expect(page.get_by_role("listbox")).to_be_visible()
            await page.screenshot(path=str(args.output / f"{artifact}-dark.png"), full_page=True)
            await field.press("Tab")
            await expect(page.get_by_role("listbox")).to_have_count(0)
            await page.get_by_role("button", name="Toggle theme", exact=True).click()
        passed("pantry/grocery dropdowns fit five viewport widths, support both themes, 44px options and Tab dismissal")
        assert not lookups, f"Warm client index should search locally: {lookups}"
        assert len(catalogs) == 1, f"Pages should share the persisted catalog: {catalogs}"

        await page.route("**/api/vocabulary/ingredients/**", lambda route: route.abort())
        await page.route("**/api/ingredient_suggestions*", lambda route: route.abort())
        await page.goto(pantry_url, wait_until="networkidle")
        await choose(page.locator("form.pantry-add").get_by_role("combobox", name="Ingredient name", exact=True), rice)
        passed("persisted client catalog remains usable when vocabulary endpoints are unavailable")

        fallback = await new_context(storage_state=await context.storage_state(), viewport={"width": 390, "height": 844})
        await fallback.route("**/api/vocabulary/ingredients/**", lambda route: route.abort())
        fallback_page = await fallback.new_page()
        fallback_page.set_default_navigation_timeout(120000)
        fallback_requests = []
        fallback_page.on("request", lambda request: fallback_requests.append(request.url) if "/api/ingredient_suggestions" in request.url else None)
        await fallback_page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
        field = fallback_page.get_by_role("combobox", name="Item name", exact=True)
        await choose(field, rice, method="mouse", target=fallback_page)
        assert fallback_requests, "Cold client should use the server fallback"
        passed("without a local catalog the authenticated server fallback still provides suggestions")
        await fallback.route("**/api/ingredient_suggestions*", lambda route: route.abort())
        await fallback_page.reload(wait_until="networkidle")
        await fallback_page.get_by_label("List name", exact=True).fill("Offline suggestions " + suffix)
        field = fallback_page.get_by_role("combobox", name="Item name", exact=True)
        await field.fill("My unusual household supplies " + suffix)
        await expect(fallback_page.get_by_role("status").filter(has_text="Suggestions unavailable")).to_be_visible()
        await fallback_page.get_by_role("button", name="Create list", exact=True).click()
        await fallback_page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        await expect(fallback_page.locator(".grocery-row")).to_contain_text("My unusual household supplies " + suffix)
        await fallback_page.goto(base + "/kitchen/pantry/new", wait_until="networkidle")
        await fallback_page.get_by_label("Pantry name", exact=True).fill("Custom pantry " + suffix)
        await fallback_page.get_by_role("combobox", name="Ingredient name", exact=True).fill("My custom spice mix " + suffix)
        await expect(fallback_page.get_by_role("status").filter(has_text="Suggestions unavailable")).to_be_visible()
        await fallback_page.get_by_role("button", name="Create pantry", exact=True).click()
        await fallback_page.wait_for_url(re.compile(r".*/kitchen/pantry/[0-9a-f-]{36}$"))
        await expect(fallback_page.locator(".pantry-row")).to_contain_text("My custom spice mix " + suffix)
        await fallback.close()
        passed("complete suggestion outage leaves custom pantry ingredients and household grocery items editable and saveable")

        # The shared component must retain recipe editor labels and UUID behavior.
        await page.unroute("**/api/vocabulary/ingredients/**")
        await page.unroute("**/api/ingredient_suggestions*")
        await page.goto(base + "/recipes/new", wait_until="networkidle")
        await page.get_by_role("textbox", name="Recipe name", exact=True).fill("Autocomplete regression " + suffix)
        await page.get_by_role("button", name="+ Add Ingredient", exact=True).click()
        field = page.get_by_role("combobox", name="Ingredient 1 name", exact=True)

        def saved_selection(response):
            if not re.search(r"/api/(create|save)_recipe_draft", response.url) or response.status != 200:
                return False
            rows = (response.request.post_data_json or {}).get("editor", {}).get("ingredients", [])
            return len(rows) == 1 and rows[0].get("vocabulary_id") == ingredients["rice"]["id"]

        async with page.expect_response(saved_selection) as saved:
            await choose(field, rice)
        rows = (await (await saved.value).json())["editor"]["ingredients"]
        assert rows[0]["name"] == rice and rows[0]["vocabulary_id"] == ingredients["rice"]["id"]
        passed("recipe editor retains its accessible labels, keyboard behavior and recognized ingredient UUID")
        assert not errors, errors
        result = {"passed": len(checks), "checks": checks, "page_errors": errors, "browser": browser.version}
        (args.output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
        await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", default="noms-groceries")
    parser.add_argument("--output", type=Path, default=Path("/workspace/noms-kitchen-autocomplete-e2e"))
    asyncio.run(main(parser.parse_args()))
