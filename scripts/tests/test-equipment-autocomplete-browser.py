#!/usr/bin/env python3
"""Equipment completion against a local mock-auth stack; private fixtures only."""
import argparse
import asyncio
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
    base = args.base_url.rstrip("/")
    assert urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    label = subprocess.check_output([
        "docker", "inspect", args.instance + "-postgres-1", "--format",
        '{{index .Config.Labels "com.noms.dev-environment"}}',
    ], text=True).strip()
    assert label == "true", "Fixtures require the named local development database"
    args.output.mkdir(parents=True, exist_ok=True)
    suffix = uuid.uuid4().hex[:10]
    checks, errors, lookups = [], [], []

    def passed(message):
        checks.append(message)
        print("PASS:", message, flush=True)

    def sql(statement, **values):
        command = ["docker", "exec", "-i", args.instance + "-postgres-1",
                   "psql", "-U", "noms", "-d", "noms", "-Atq", "-v", "ON_ERROR_STOP=1"]
        for key, value in values.items():
            command += ["-v", key + "=" + value]
        return subprocess.run(command, input=statement, text=True, capture_output=True, check=True).stdout.strip()

    async with async_playwright() as playwright:
        options = dict(executable_path=args.chromium, headless=True,
                       args=["--disable-dev-shm-usage"])
        proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("HTTP_PROXY")
        if proxy:
            options["proxy"] = dict(server=proxy, bypass="127.0.0.1,localhost,[::1]")
        browser = await playwright.chromium.launch(**options)
        context = await browser.new_context(viewport={"width": 1440, "height": 1000}, has_touch=True)
        page = await context.new_page()
        try:
            await context.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
            page.on("pageerror", lambda error: errors.append(str(error)))
            page.on("request", lambda request: lookups.append(request.url)
                    if re.search(r"/api/[^/?]*equipment", request.url) else None)
            await page.goto(base + "/login", wait_until="networkidle")
            await page.get_by_role("link", name="Continue with Google").click()
            assert urlsplit(page.url).hostname in {"127.0.0.1", "localhost"}
            await page.locator("input[name=username]").fill("equipment-" + suffix)
            await page.locator("textarea[name=claims]").fill(json.dumps({
                "email": f"equipment-{suffix}@example.test", "email_verified": True,
                "name": "Equipment Suggestions Test",
            }))
            await page.locator("input[type=submit]").click()
            await page.wait_for_url(base + "/**")
            profile = await (await context.request.get(base + "/api/user_profile")).json()
            owner = str(uuid.UUID(profile["current_user_id"]))

            def field(index):
                return page.get_by_role("combobox", name=f"Equipment {index} name", exact=True)

            async def choose(index, query, expected, method="keyboard"):
                target = field(index)
                await target.fill(query)
                option = page.get_by_role("option", name=expected, exact=True)
                await expect(option).to_be_visible()
                controls = await target.get_attribute("aria-controls")
                menu = page.locator(f'[id="{controls}"]')
                assert await menu.count() == 1
                if method == "keyboard":
                    names = await menu.get_by_role("option").all_text_contents()
                    for _ in range(names.index(expected) + 1):
                        await target.press("ArrowDown")
                    await expect(option).to_have_attribute("aria-selected", "true")
                    assert await target.get_attribute("aria-activedescendant") == await option.get_attribute("id")
                    await target.press("Enter")
                elif method == "touch":
                    await option.tap()
                else:
                    await option.click()
                await expect(target).to_have_value(expected)
                await expect(target).to_have_attribute("aria-expanded", "false")

            async def wait_saved(draft_id, names):
                for _ in range(80):
                    value = sql("SELECT snapshot->'equipment' FROM recipe_drafts WHERE id=:'draft'::uuid AND user_id=:'owner'::uuid;",
                                draft=draft_id, owner=owner)
                    if value and [row["name"] for row in json.loads(value)] == names:
                        return
                    await asyncio.sleep(0.25)
                raise AssertionError("Equipment edits were not persisted")

            await page.goto(base + "/recipes/new", wait_until="networkidle")
            await page.get_by_role("button", name="+ Add Equipment", exact=True).click()
            await choose(1, "pizza steel", "baking steel")
            await page.get_by_role("button", name="+ Add Equipment", exact=True).click()
            assert await field(1).get_attribute("aria-controls") != await field(2).get_attribute("aria-controls")
            await field(2).fill("mixer")
            await expect(page.get_by_role("option", name="hand mixer", exact=True)).to_be_visible()
            await expect(page.get_by_role("option", name="stand mixer", exact=True)).to_be_visible()
            await field(2).press("Escape")
            await expect(field(2)).to_have_value("mixer")
            await expect(field(2)).to_have_attribute("aria-expanded", "false")
            await field(2).fill("my family heirloom pan")
            await field(2).press("Tab")
            await expect(field(2)).to_have_value("my family heirloom pan")
            await expect(field(2)).to_have_attribute("aria-expanded", "false")
            await field(1).fill("pizza steel")
            await field(1).press("Enter")
            await expect(field(1)).to_have_value("pizza steel")
            await field(1).press("Tab")
            await expect(field(1)).to_have_value("pizza steel")
            await choose(1, "pizza steel", "baking steel", "mouse")
            passed("new recipe: aliases, keyboard/mouse selection, distinct broad-term targets, unique IDs and free-text dismissal")

            await page.get_by_label("Recipe name", exact=True).fill("Equipment draft " + suffix)
            await page.wait_for_url(re.compile(r".*/recipes/drafts/[0-9a-f-]{36}/edit$"))
            draft_id = page.url.split("/")[-2]
            await wait_saved(draft_id, ["baking steel", "my family heirloom pan"])
            await page.reload(wait_until="networkidle")
            await expect(field(1)).to_have_value("baking steel")
            await expect(field(2)).to_have_value("my family heirloom pan")
            await choose(2, "baking paper", "parchment paper")
            await wait_saved(draft_id, ["baking steel", "parchment paper"])
            passed("draft editing: preferred labels, custom names and related supplies survive save/reload")

            await page.set_viewport_size({"width": 390, "height": 900})
            # Contents navigation activates the optional panel on small screens.
            await page.locator('.recipe-editor__contents a[href="#recipe-equipment"]').click()
            await expect(page.locator('#recipe-equipment')).to_be_focused()
            await choose(1, "pastry sheeter", "dough sheeter", "touch")
            await field(2).fill("pan")
            await field(2).press("ArrowUp")
            active = await field(2).get_attribute("aria-activedescendant")
            selected = page.locator(f'[id="{active}"]')
            await expect(selected).to_be_visible()
            controls = await field(2).get_attribute("aria-controls")
            menu = page.locator(f'[id="{controls}"]')
            selected_box, menu_box = await selected.bounding_box(), await menu.bounding_box()
            assert selected_box["y"] >= menu_box["y"]
            assert selected_box["y"] + selected_box["height"] <= menu_box["y"] + menu_box["height"]
            await field(2).press("Escape")
            await expect(field(2)).to_have_value("pan")
            await field(2).fill("mixer")
            await expect(page.get_by_role("option", name="stand mixer", exact=True)).to_be_visible()
            await page.screenshot(path=str(args.output / "equipment-mobile.png"), full_page=True)
            await field(2).press("Escape")
            await expect(field(2)).to_have_value("mixer")
            await choose(2, "baking paper", "parchment paper", "touch")
            await page.locator('.recipe-editor__equipment-row').first.get_by_role("button", name="Remove item", exact=True).click()
            await expect(field(1)).to_have_value("parchment paper")
            await expect(field(2)).to_have_count(0)
            await wait_saved(draft_id, ["parchment paper"])
            passed("mobile: touch selection, keyboard navigation, dismissal, row removal and persistence")

            recipe_id = str(uuid.uuid4())
            sql("""BEGIN;
                INSERT INTO recipes(id,user_id,title,visibility,equipment,ingredients,instructions)
                VALUES(:'recipe'::uuid,:'owner'::uuid,'Private equipment fixture','private',
                       '[{"name":"Grandmother pot"}]'::jsonb,'[]'::jsonb,'[]'::jsonb);
                INSERT INTO recipe_versions(recipe_id,version_number,snapshot)
                SELECT id,1,jsonb_build_object('title',title,'description',description,'commentary',commentary,
                    'ingredients',ingredients,'instructions',instructions,'equipment',equipment,'images',images,'tags','[]'::jsonb)
                FROM recipes WHERE id=:'recipe'::uuid;
                COMMIT;""", recipe=recipe_id, owner=owner)
            await page.set_viewport_size({"width": 1440, "height": 1000})
            await page.goto(base + f"/recipes/{recipe_id}/edit", wait_until="networkidle")
            await expect(field(1)).to_have_value("Grandmother pot")
            await choose(1, "pepper grinder", "pepper mill")
            for _ in range(80):
                saved = sql("SELECT snapshot->'equipment' FROM recipe_drafts WHERE recipe_id=:'recipe'::uuid AND user_id=:'owner'::uuid;",
                            recipe=recipe_id, owner=owner)
                if saved and json.loads(saved) == [{"name": "pepper mill"}]:
                    break
                await asyncio.sleep(0.25)
            else:
                raise AssertionError("Existing recipe draft did not save")
            assert json.loads(sql("SELECT equipment FROM recipes WHERE id=:'recipe'::uuid;", recipe=recipe_id)) == [{"name": "Grandmother pot"}]
            await page.reload(wait_until="networkidle")
            await expect(field(1)).to_have_value("pepper mill")
            passed("existing recipe: historical text preserved until explicit choice; draft persists without changing the published recipe")

            # No server lookup is available during this last selection.
            await context.set_offline(True)
            await choose(1, "idiyappam press", "string hopper press")
            await field(1).fill("offline custom equipment")
            await field(1).press("Tab")
            await expect(field(1)).to_have_value("offline custom equipment")
            await context.set_offline(False)
            assert not lookups, lookups
            assert not errors, errors
            passed("matching and custom typing work offline with no equipment lookup requests or page errors")
            await field(1).fill("mixer")
            await expect(page.get_by_role("option", name="hand mixer", exact=True)).to_be_visible()
            await page.screenshot(path=str(args.output / "equipment-desktop.png"), full_page=True)
            (args.output / "result.json").write_text(json.dumps(dict(checks=checks, errors=errors, equipment_requests=lookups), indent=2))
        except Exception:
            await page.screenshot(path=str(args.output / "failure.png"), full_page=True)
            raise
        finally:
            await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", required=True)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--chromium", default="/usr/bin/chromium")
    parser.add_argument("--output", type=Path, default=Path("/tmp/noms-equipment-autocomplete-browser"))
    asyncio.run(main(parser.parse_args()))
