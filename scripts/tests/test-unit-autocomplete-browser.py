#!/usr/bin/env python3
"""Precise units in a ready local mock-auth stack; creates only private fixtures."""
import argparse
import asyncio
import copy
import hashlib
import json
import os
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import urlsplit


async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=20000)
    assert re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance)
    if args.base_url:
        base = args.base_url.rstrip("/")
    else:
        metadata = dict(line.split("=", 1) for line in
                        (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
        base = metadata["APP_URL"]
    assert urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    label = subprocess.run(["docker", "inspect", args.instance + "-postgres-1", "--format",
                            '{{index .Config.Labels "com.noms.dev-environment"}}'],
                           text=True, capture_output=True, check=True).stdout.strip()
    assert label == "true", "Raw fixtures require the named local development database"
    args.output.mkdir(parents=True, exist_ok=True)
    suffix = uuid.uuid4().hex[:10]
    checks, errors, requests, autosaved_units = [], [], [], []

    def passed(message):
        checks.append(message)
        print("PASS:", message, flush=True)

    async with async_playwright() as playwright:
        options = dict(executable_path="/usr/bin/chromium", headless=True, args=["--disable-dev-shm-usage"])
        proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("HTTP_PROXY")
        if proxy:
            options["proxy"] = dict(server=proxy, bypass="127.0.0.1,localhost,[::1]")
        browser = await playwright.chromium.launch(**options)
        context = await browser.new_context(viewport={"width": 1440, "height": 1000}, has_touch=True)
        try:
            await context.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
            page = await context.new_page()
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.on("request", lambda request: requests.append(request.url))
            def record_autosave(request):
                if request.method == "POST" and re.search(r"/api/(?:create|save)_recipe_draft", request.url):
                    payload = request.post_data_json or {}
                    autosaved_units.extend(row["unit"] for row in payload.get("editor", {}).get("ingredients", []))
            page.on("request", record_autosave)
            await page.goto(base + "/login", wait_until="networkidle")
            await page.get_by_role("link", name="Continue with Google").click()
            assert urlsplit(page.url).hostname in {"127.0.0.1", "localhost"}
            await page.locator("input[name=username]").fill("units-" + suffix)
            await page.locator("textarea[name=claims]").fill(json.dumps({
                "email": f"units-{suffix}@example.test", "email_verified": True, "name": "Unit Suggestions Test"}))
            await page.locator("input[type=submit]").click()
            await page.wait_for_url(base + "/**")
            profile = await (await context.request.get(base + "/api/user_profile")).json()
            owner = str(uuid.UUID(profile["current_user_id"]))

            async def choose(field, query, option_name, saved, method="keyboard"):
                await field.fill(query)
                option = page.get_by_role("option", name=option_name, exact=True)
                await expect(option).to_be_visible()
                controls = await field.get_attribute("aria-controls")
                menu = page.locator(f'[id="{controls}"]')
                if method == "keyboard":
                    names = await menu.get_by_role("option").evaluate_all("rows => rows.map(r => r.getAttribute('aria-label'))")
                    for _ in range(names.index(option_name) + 1):
                        await field.press("ArrowDown")
                    await expect(option).to_have_attribute("aria-selected", "true")
                    assert await field.get_attribute("aria-activedescendant") == await option.get_attribute("id")
                    await field.press("Enter")
                elif method == "touch":
                    await option.tap()
                else:
                    await option.click()
                await expect(field).to_have_value(saved)
                await expect(field).to_have_attribute("aria-expanded", "false")

            await page.goto(base + "/recipes/new", wait_until="networkidle")
            await page.get_by_role("button", name="+ Add Ingredient", exact=True).click()
            unit = page.get_by_role("combobox", name="Ingredient 1 unit", exact=True)
            await page.get_by_role("combobox", name="Ingredient 1 name", exact=True).fill("Flour for units test")
            await page.get_by_label("Ingredient 1 amount", exact=True).fill("1/2")
            await choose(unit, "grammes", "gram", "g")
            await unit.fill("grammes")
            await unit.press("Enter")
            await expect(unit).to_have_value("g")
            await expect(page.get_by_label("Ingredient 1 amount", exact=True)).to_have_value("1/2")
            await expect(page.get_by_role("combobox", name="Ingredient 1 name", exact=True)).to_have_value("Flour for units test")
            await unit.fill("cups")
            await expect(page.get_by_role("option", name="US customary cup", exact=True)).to_be_visible()
            await expect(page.get_by_role("option", name="metric cup (250 mL)", exact=True)).to_be_visible()
            await unit.press("Escape")
            await expect(unit).to_have_value("US cup")
            await expect(unit).to_have_attribute("aria-expanded", "false")
            await unit.fill("oz")
            await expect(page.get_by_role("option", name="ounce (weight)", exact=True)).to_be_visible()
            await expect(page.get_by_role("option", name=re.compile("fluid ounce"))).to_have_count(0)
            await unit.press("Tab")
            await expect(unit).to_have_value("oz")
            await expect(unit).to_have_attribute("aria-expanded", "false")
            for text in ["pinch", "dash", "handful", "clove", "can", "packet"]:
                await unit.fill(text)
                await expect(unit).to_have_attribute("aria-expanded", "false")
            await page.get_by_role("button", name="+ Add Ingredient", exact=True).click()
            other = page.get_by_role("combobox", name="Ingredient 2 unit", exact=True)
            assert await unit.get_attribute("aria-controls") != await other.get_attribute("aria-controls")
            await expect(other).to_have_value("")
            await other.focus()
            await other.press("ArrowUp")
            active_id = await other.get_attribute("aria-activedescendant")
            active_option = page.locator(f'[id="{active_id}"]')
            await expect(active_option).to_be_visible()
            menu = page.get_by_role("listbox", name="Unit suggestions", exact=True)
            active_box, menu_box = await active_option.bounding_box(), await menu.bounding_box()
            assert active_box["y"] >= menu_box["y"] and active_box["y"] + active_box["height"] <= menu_box["y"] + menu_box["height"]
            await other.press("Enter")
            await expect(other).to_have_value("oz")
            await other.fill("")
            await other.press("Tab")
            await expect(other).to_have_value("")
            await unit.fill("family measure")
            await unit.press("Tab")
            await expect(unit).to_have_attribute("aria-invalid", "true")
            await expect(page.get_by_role("alert").filter(has_text="supported weight or volume")).to_be_visible()
            await asyncio.sleep(2.5)
            await expect(page.get_by_text("Could not confirm draft creation", exact=False)).to_have_count(0)
            await unit.fill("milliliters")
            await unit.press("Tab")
            await expect(unit).to_have_value("mL")
            await page.get_by_label("Recipe name", exact=True).fill("Units draft " + suffix)
            await page.wait_for_url(re.compile(r".*/recipes/drafts/[0-9a-f-]{36}/edit$"))
            await asyncio.sleep(3)
            await page.reload(wait_until="networkidle")
            unit = page.get_by_role("combobox", name="Ingredient 1 unit", exact=True)
            await expect(unit).to_have_value("mL")
            await expect(page.get_by_role("combobox", name="Ingredient 2 unit", exact=True)).to_have_value("")
            await choose(unit, "milliliters", "millilitre", "mL", method="mouse")
            await asyncio.sleep(3)
            await page.reload(wait_until="networkidle")
            await expect(page.get_by_role("combobox", name="Ingredient 1 unit", exact=True)).to_have_value("mL")
            await unit.fill("kilograms")
            await page.get_by_label("Ingredient 1 amount", exact=True).click()
            await expect(unit).to_have_value("kg")
            passed("recipe Enter/Tab/Escape/blur defaults, explicit regional choices, invalid-unit feedback and draft save/reload")

            # Exercise the published-recipe editor independently of the draft form.
            recipe_id = str(uuid.uuid4())
            fixture = json.dumps([{"name": "Test rice", "amount": "2", "unit": "cups"}])
            sql = """BEGIN;
                INSERT INTO recipes(id,user_id,title,visibility,ingredients,instructions)
                VALUES(:'recipe'::uuid,:'owner'::uuid,'Private units fixture','private',:'ingredients'::jsonb,'[]'::jsonb);
                INSERT INTO recipe_versions(recipe_id,version_number,snapshot)
                SELECT id,1,jsonb_build_object('title',title,'description',description,
                    'commentary',commentary,'ingredients',ingredients,'instructions',instructions,
                    'equipment',equipment,'images',images,'tags','[]'::jsonb)
                FROM recipes WHERE id=:'recipe'::uuid;
                COMMIT;"""
            subprocess.run(["docker", "exec", "-i", args.instance + "-postgres-1", "psql", "-U", "noms", "-d", "noms", "-v", "ON_ERROR_STOP=1", "-v", "recipe=" + recipe_id, "-v", "owner=" + owner, "-v", "ingredients=" + fixture], input=sql, text=True, capture_output=True, check=True)
            await page.goto(base + f"/recipes/{recipe_id}/edit", wait_until="networkidle")
            unit = page.get_by_role("combobox", name="Ingredient 1 unit", exact=True)
            await expect(unit).to_have_value("cups")
            await choose(unit, "pounds", "pound", "lb")
            await expect(page.get_by_label("Ingredient 1 amount", exact=True)).to_have_value("2")
            passed("existing recipe edit preserves authored units on load and supports explicit selection")

            await page.goto(base + "/kitchen/pantry/new", wait_until="networkidle")
            await page.get_by_label("Pantry name", exact=True).fill("Units pantry " + suffix)
            add = page.locator("form.pantry-add")
            await add.get_by_role("combobox", name="Ingredient name", exact=True).fill("Unit test rice")
            await add.get_by_label("Amount", exact=True).fill("3")
            unit = add.get_by_role("combobox", name="Unit", exact=True)
            await choose(unit, "cups metric", "metric cup (250 mL)", "metric cup")
            await expect(page.locator(".pantry-draft-row")).to_have_count(0)
            await add.get_by_role("button", name="Add ingredient", exact=True).click()
            await page.get_by_role("button", name="Edit initial Unit test rice", exact=True).click()
            dialog = page.get_by_role("dialog")
            unit = dialog.get_by_role("combobox", name="Unit", exact=True)
            await unit.fill("litres")
            await unit.press("Escape")
            await expect(dialog).to_be_visible()
            await choose(unit, "litres", "litre", "L")
            await dialog.get_by_role("button", name="Save item", exact=True).click()
            await page.get_by_role("button", name="Create pantry", exact=True).click()
            await page.wait_for_url(re.compile(r".*/kitchen/pantry/[0-9a-f-]{36}$"))
            await page.reload(wait_until="networkidle")
            await expect(page.locator(".pantry-row")).to_contain_text("3 L")
            pantry_url = page.url
            passed("pantry create/edit modal, Escape without closing dialog, no accidental form submission and saved units")

            await page.set_viewport_size({"width": 390, "height": 1000})
            await page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
            await page.get_by_label("List name", exact=True).fill("Units groceries " + suffix)
            add = page.locator("form.grocery-add-row")
            await add.get_by_role("combobox", name="Item name", exact=True).fill("Unit test sugar")
            await add.get_by_label("Amount", exact=True).fill("4")
            await choose(add.get_by_role("combobox", name="Unit", exact=True), "fl. oz.", "US fluid ounce", "US fl oz", method="touch")
            await add.get_by_role("button", name="Add", exact=True).click()
            await page.get_by_role("button", name="Create list", exact=True).click()
            await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
            grocery_url = page.url
            row = page.locator(".grocery-row").filter(has_text="Unit test sugar")
            await expect(row).to_contain_text("4 US fl oz")
            await row.get_by_role("button", name="Edit Unit test sugar", exact=True).click()
            dialog = page.get_by_role("dialog")
            await choose(dialog.get_by_role("combobox", name="Unit", exact=True), "ounces", "ounce (weight)", "oz")
            await dialog.get_by_role("button", name="Save item", exact=True).click()
            await page.reload(wait_until="networkidle")
            await expect(page.locator(".grocery-row").filter(has_text="Unit test sugar")).to_contain_text("4 oz")
            passed("grocery create/edit, touch selection and distinct weight/fluid-ounce persistence")

            # Bypass the combobox and exercise the real authenticated API. The
            # same exact alias rules must apply, with no partial writes on error.
            log = subprocess.run(["docker", "logs", args.instance + "-app-1"],
                                 text=True, capture_output=True, check=True)
            registered = re.findall(r"Registering: POST (/api/[^\s\x1b]+)", log.stdout + log.stderr)

            async def api(name, payload, client=context):
                paths = [p for p in registered if re.fullmatch(r"/api/" + name + r"[0-9]*", p)]
                assert paths, name
                return await client.request.post(base + paths[-1], data=json.dumps(payload),
                                                headers={"Content-Type": "application/json", "Origin": base})

            def fingerprint():
                tables = ["recipes", "recipe_drafts", "recipe_versions", "kitchen_profiles",
                          "pantry_ingredients", "pantry_equipment", "grocery_lists", "grocery_list_items"]
                sql = "BEGIN READ ONLY;" + "".join(
                    "SELECT count(*),md5(coalesce(string_agg(to_jsonb(t)::text,'' ORDER BY to_jsonb(t)::text),'')) FROM " + table + " t;"
                    for table in tables) + "COMMIT;"
                return subprocess.run(["docker", "exec", "-i", args.instance + "-postgres-1",
                                       "psql", "-U", "noms", "-d", "noms", "-Atq", "-v", "ON_ERROR_STOP=1"],
                                      input=sql, text=True, capture_output=True, check=True).stdout

            async def rejected(name, payload):
                before = fingerprint()
                response = await api(name, payload)
                text = await response.text()
                assert not response.ok and "supported weight or volume" in text, (name, response.status, text)
                assert fingerprint() == before, (name, "rejected request changed stored data")

            row = {"name": "API unit ingredient", "amount": "1/2", "unit": "grams"}
            editor = {"title": "API units " + suffix, "description": None, "commentary": None,
                      "prep_time_minutes": None, "cook_time_minutes": None, "servings": 2,
                      "ingredients": [row], "instructions": [{"text": "Mix.", "sub_steps": [], "photos": []}],
                      "equipment": [], "images": [], "tags": [], "visibility": "private"}
            bad = copy.deepcopy(editor)
            bad["ingredients"].append({**row, "unit": "pinch"})
            await rejected("create_recipe_draft", {"expected_account_id": owner, "editor": bad})
            await rejected("create_recipe", {"expected_account_id": owner, **bad})
            response = await api("create_recipe_draft", {"expected_account_id": owner, "editor": editor})
            assert response.ok, await response.text()
            draft = await response.json()
            assert draft["editor"]["ingredients"][0]["unit"] == "g"
            await rejected("save_recipe_draft", {"expected_account_id": owner, "draft_id": draft["id"],
                                                "expected_revision": draft["revision"], "editor": bad})
            edited = copy.deepcopy(editor)
            edited["ingredients"][0]["unit"] = "UK pints"
            response = await api("save_recipe_draft", {"expected_account_id": owner, "draft_id": draft["id"],
                                 "expected_revision": draft["revision"], "editor": edited})
            assert response.ok, await response.text()
            assert (await response.json())["editor"]["ingredients"][0]["unit"] == "Imperial pt"
            response = await api("create_recipe", {"expected_account_id": owner, **editor})
            assert response.ok, await response.text()
            created_recipe = await response.json()
            assert created_recipe["ingredients"][0]["unit"] == "g"
            await rejected("update_recipe", {"expected_account_id": owner, "recipe_id": created_recipe["id"],
                                             "ingredients": bad["ingredients"]})
            response = await api("update_recipe", {"expected_account_id": owner, "recipe_id": created_recipe["id"],
                                 "ingredients": [{**row, "unit": "liters"}]})
            assert response.ok, await response.text()
            assert (await response.json())["ingredients"][0]["unit"] == "L"

            pantry_id = pantry_url.rsplit("/", 1)[1]
            stock = {"fields": row, "available": True}
            await rejected("create_kitchen", {"expected_account_id": owner, "request": {
                "request_id": str(uuid.uuid4()), "name": "Invalid units", "note": "", "copy": None,
                "ingredients": [stock, {"fields": {**row, "unit": "can"}, "available": True}], "equipment": []}})
            await rejected("add_pantry_entry", {"expected_account_id": owner, "profile_id": pantry_id,
                "kind": "Ingredient", "request_id": str(uuid.uuid4()),
                "stock": {"fields": {**row, "unit": "gra"}, "available": True}})
            response = await api("add_pantry_entry", {"expected_account_id": owner, "profile_id": pantry_id,
                "kind": "Ingredient", "request_id": str(uuid.uuid4()), "stock": stock})
            assert response.ok, await response.text()
            kitchen = await (await api("get_kitchen", {"expected_account_id": owner, "id": pantry_id})).json()
            item = next(x for x in kitchen["entries"] if x["stock"]["fields"]["name"] == row["name"])
            assert item["stock"]["fields"]["unit"] == "g"
            await rejected("update_pantry_entry", {"expected_account_id": owner, "profile_id": pantry_id,
                "kind": "Ingredient", "item_id": item["id"], "expected_revision": item["revision"],
                "change": {"Fields": {**row, "unit": "packet"}}})

            grocery_id = grocery_url.rsplit("/", 1)[1]
            await rejected("create_grocery_list", {"expected_account_id": owner, "request": {
                "request_id": str(uuid.uuid4()), "name": "Invalid units", "note": "", "recipes": [],
                "manual_items": [row, {**row, "unit": "to taste"}]}})
            await rejected("add_grocery_item", {"expected_account_id": owner, "list_id": grocery_id,
                "request_id": str(uuid.uuid4()), "fields": {**row, "unit": "unknown"}})
            response = await api("add_grocery_item", {"expected_account_id": owner, "list_id": grocery_id,
                "request_id": str(uuid.uuid4()), "fields": {**row, "unit": "metric cups"}})
            assert response.ok, await response.text()
            grocery = await (await api("get_grocery_list", {"expected_account_id": owner, "list_id": grocery_id})).json()
            item = next(x for x in grocery["items"] if x["fields"]["name"] == row["name"])
            assert item["fields"]["unit"] == "metric cup"
            await rejected("update_grocery_item", {"expected_account_id": owner, "list_id": grocery_id,
                "item_id": item["id"], "expected_revision": item["revision"],
                "change": {"Fields": {**row, "unit": "boxes"}}})

            response = await api("export_recipe_json", {"recipe_id": created_recipe["id"]})
            assert response.ok, await response.text()
            portable = json.loads(await response.json())
            portable["snapshot"]["ingredients"][0]["unit"] = "not a measure"
            encoded = json.dumps(portable["snapshot"], sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
            portable["snapshot_digest"] = "sha256:" + hashlib.sha256(encoded).hexdigest()
            await rejected("import_recipe_json", {"expected_account_id": owner, "json": json.dumps(portable),
                                                   "available_external_urls": []})
            portable["snapshot"]["ingredients"][0]["unit"] = "grammes"
            encoded = json.dumps(portable["snapshot"], sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
            portable["snapshot_digest"] = "sha256:" + hashlib.sha256(encoded).hexdigest()
            response = await api("import_recipe_json", {"expected_account_id": owner, "json": json.dumps(portable),
                                 "available_external_urls": []})
            assert response.ok, await response.text()
            assert (await response.json())["editor"]["ingredients"][0]["unit"] == "g"
            anonymous = await browser.new_context()
            before = fingerprint()
            response = await api("create_recipe_draft", {"expected_account_id": owner, "editor": editor}, anonymous)
            assert not response.ok
            assert fingerprint() == before
            await anonymous.close()
            passed("direct recipe/draft/import, pantry and grocery APIs standardize aliases, reject invalid units atomically and require authentication")

            for url, field_name, scope, artifact in [
                (base + "/recipes/new", "Ingredient 1 unit", None, "recipe"),
                (pantry_url, "Unit", "form.pantry-add", "pantry"),
                (grocery_url, "Unit", "form.grocery-add-row", "grocery"),
            ]:
                await page.set_viewport_size({"width": 1440, "height": 1000})
                await page.goto(url, wait_until="networkidle")
                if artifact == "recipe":
                    await page.get_by_role("button", name="+ Add Ingredient", exact=True).click()
                target = page.locator(scope) if scope else page
                field = target.get_by_role("combobox", name=field_name, exact=True)
                for width in [320, 390, 760, 1440]:
                    await page.set_viewport_size({"width": width, "height": 1000})
                    await field.fill("tablespoons")
                    await field.evaluate("el => el.scrollIntoView({block:'center',behavior:'instant'})")
                    menu = page.get_by_role("listbox", name="Unit suggestions", exact=True)
                    await expect(menu).to_be_visible()
                    box = await menu.bounding_box()
                    field_box = await field.bounding_box()
                    assert box and box["x"] >= 0 and box["x"] + box["width"] <= width + 1, (artifact, width, box)
                    assert box["height"] <= 162, (artifact, width, "unit menu must stay compact", box)
                    assert field_box and 0 <= box["y"] - (field_box["y"] + field_box["height"]) <= 12, (artifact, width, "menu disconnected from input", field_box, box)
                    assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), (artifact, width)
                    row_height = await menu.get_by_role("option").first.evaluate("el => el.getBoundingClientRect().height")
                    minimum = 44 if await page.evaluate("matchMedia('(pointer: coarse)').matches") else 32
                    assert minimum <= row_height <= 44, (artifact, width, row_height)
                    if width == 390:
                        await page.screenshot(path=str(args.output / f"{artifact}-390.png"), full_page=True)
                    await field.press("Tab")
                    await expect(field).to_have_attribute("aria-expanded", "false")
            passed("compact recipe/pantry/grocery menus fit four viewport widths with 32px desktop / 44px coarse-pointer options")
            assert not [url for url in requests if re.search(r"/api/(?:unit_suggestions|vocabulary/units)", url)], "Unit suggestions must not make API calls"
            assert not errors, errors
            assert not set(autosaved_units) & {"pinch", "dash", "handful", "clove", "can", "packet", "family measure"}, autosaved_units
            result = {"passed": checks, "page_errors": errors, "unit_api_requests": 0}
            (args.output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps(result), flush=True)
        except Exception:
            await page.screenshot(path=str(args.output / "failure.png"), full_page=True)
            raise
        finally:
            await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", required=True)
    parser.add_argument("--base-url", help="Explicit loopback URL when runtime metadata is unavailable")
    parser.add_argument("--output", type=Path, required=True)
    asyncio.run(main(parser.parse_args()))
