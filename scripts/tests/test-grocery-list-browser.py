#!/usr/bin/env python3
"""Local mock-auth grocery E2E. Uses a disposable Noms development instance only.

python3 scripts/tests/test-grocery-list-browser.py --instance noms-groceries --output /tmp/grocery-e2e
Requires Chromium, Python Playwright, and access to the managed Docker socket.
Fixtures use synthetic accounts; browser cookies are never printed or committed.
"""
import argparse
import asyncio
import json
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import urlsplit


def sql(instance, statement):
    result = subprocess.run(
        ["docker", "exec", "-i", f"{instance}-postgres-1", "sh", "-c",
         'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -At'],
        input=statement, text=True, capture_output=True, check=True)
    return result.stdout.strip()


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


async def main(args):
    from playwright.async_api import async_playwright, expect
    if not re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance):
        raise RuntimeError("Expected an exact local Noms development instance name")
    values = dict(line.split("=", 1) for line in
                  (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
    base = values["APP_URL"]
    if values.get("STATE") != "ready" or urlsplit(base).hostname not in {"127.0.0.1", "localhost", "::1"}:
        raise RuntimeError("Run only against a ready loopback development instance")
    args.output.mkdir(parents=True, exist_ok=True)
    suffix = uuid.uuid4().hex[:10]
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])
        context = await browser.new_context(viewport={"width": 1440, "height": 1000})
        page = await context.new_page()
        errors = []
        page.on("pageerror", lambda e: errors.append(str(e)))
        # This is the development stack's synthetic OAuth provider, never a real account.
        await page.goto(base + "/login", wait_until="networkidle")
        await page.get_by_role("link", name="Continue with Google").click()
        if not await page.locator('input[name="username"]').count():
            raise RuntimeError("Expected the local mock OAuth form")
        await page.locator('input[name="username"]').fill("grocery-e2e-" + suffix)
        await page.locator('textarea[name="claims"]').fill(json.dumps({
            "email": f"grocery-{suffix}@example.test", "email_verified": True,
            "name": "Grocery Demo"}))
        await page.locator('input[type="submit"]').click()
        await page.wait_for_url(base + "/**")
        profile = await (await context.request.get(base + "/api/user_profile")).json()
        owner = str(uuid.UUID(profile["current_user_id"]))
        other = str(uuid.uuid4())
        ids = {key: str(uuid.uuid4()) for key in ("own", "public", "unlisted", "private", "unknown")}
        statements = [f"INSERT INTO users(id,username,display_name,email) VALUES('{other}','source_{suffix}','Recipe Source Demo','source-{suffix}@example.test');"]
        for key, title, actor, visibility, amount, unit, servings in [
            ("own", "E2E chickpea curry", owner, "private", "1", "cup", "2"),
            ("public", "E2E rice bowls", other, "public", "2", "cup", "2"),
            ("unlisted", "E2E shared supper", other, "unlisted", "3", "cups", "2"),
            ("private", "E2E hidden recipe", other, "private", "8", "cup", "2"),
            ("unknown", "E2E original portions", owner, "private", "5", "g", "NULL"),
        ]:
            ingredients = json.dumps([
                {"name": "Rice" if key != "unknown" else "Salt", "amount": amount, "unit": unit},
                {"name": "Garnish (optional)", "amount": "to taste", "unit": ""},
            ])
            statements.append(f"INSERT INTO recipes(id,user_id,title,servings,ingredients,visibility) VALUES('{ids[key]}','{actor}',{literal(title+" "+suffix)},{servings},{literal(ingredients)}::jsonb,'{visibility}');")
        sql(args.instance, "\n".join(statements))
        # Manual list, adding the unsubmitted entry is deliberate and lossless.
        await page.goto(base + "/grocery-lists/new", wait_until="networkidle")
        await page.get_by_label("List name", exact=True).fill("Household essentials")
        form = page.locator(".grocery-add-row")
        await form.get_by_label("Item name", exact=True).fill("Dish soap")
        await form.get_by_label("Amount", exact=True).fill("1")
        await form.get_by_label("Unit", exact=True).fill("bottle")
        await page.get_by_role("button", name="Create list", exact=True).click()
        await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        manual_url = page.url
        await expect(page.locator("h1")).to_have_text("Household essentials")
        await expect(page.locator(".grocery-row")).to_have_count(1)
        await expect(page.locator(".grocery-inspector")).to_have_count(0)
        check = page.locator('input[type="checkbox"]').first
        # Failed writes must roll back the optimistic checkbox.
        await page.route("**/api/update_grocery_item*", lambda route: route.abort())
        await check.click()
        await expect(page.locator(".grocery-error").first).to_be_visible()
        await expect(check).not_to_be_checked()
        await page.unroute("**/api/update_grocery_item*")
        await check.click()
        await expect(check).to_be_checked()
        await expect(check).to_be_enabled()
        await page.reload(wait_until="networkidle")
        await expect(page.locator('input[type="checkbox"]').first).to_be_checked()
        # A second browser session reads committed database state.
        second = await browser.new_context(storage_state=await context.storage_state())
        second_page = await second.new_page()
        await second_page.goto(manual_url, wait_until="networkidle")
        await expect(second_page.locator('input[type="checkbox"]').first).to_be_checked()
        await second.close()

        await page.goto(base + "/grocery-lists/new", wait_until="networkidle")
        await page.get_by_label("List name", exact=True).fill("This week’s groceries")
        await page.get_by_role("button", name="From recipes").click()
        await expect(page.locator(".grocery-recipe-options")).to_be_visible()
        await page.get_by_role("searchbox", name="Search accessible recipes").fill(suffix)
        await page.locator(".grocery-recipe-search").first.get_by_role("button", name="Search", exact=True).click()
        await expect(page.locator(".grocery-recipe-options")).not_to_contain_text("E2E hidden recipe")
        await expect(page.locator(".grocery-recipe-options")).not_to_contain_text("E2E shared supper")
        for title in ("E2E chickpea curry", "E2E rice bowls"):
            row = page.locator(".grocery-recipe-option").filter(has_text=title+" "+suffix)
            await row.get_by_role("button", name="Add", exact=True).click()
            await expect(page.locator(".grocery-selected-recipe").filter(has_text=title)).to_be_visible()
        selected = page.locator(".grocery-selected-recipe").filter(has_text="E2E chickpea curry")
        await selected.get_by_role("spinbutton").fill("0")
        await page.get_by_role("button", name="Create list", exact=True).click()
        await expect(page.locator(".grocery-error")).to_contain_text("Target servings")
        await selected.get_by_role("spinbutton").fill("4")
        ref = page.get_by_label("Accessible recipe link or ID", exact=True)
        await ref.fill(ids["private"])
        await page.get_by_role("button", name="Add link", exact=True).click()
        await expect(page.locator(".grocery-error")).to_contain_text("Recipe not accessible")
        await ref.fill(base + "/recipes/" + ids["unlisted"])
        await page.get_by_role("button", name="Add link", exact=True).click()
        await expect(page.locator(".grocery-selected-recipe")).to_have_count(3)
        # Removing/reselecting must reset both displayed and effective servings.
        await selected.get_by_role("button", name=re.compile("^Remove E2E chickpea curry")).click()
        await page.locator(".grocery-recipe-option").filter(has_text="E2E chickpea curry "+suffix).get_by_role("button", name="Add", exact=True).click()
        await expect(selected.get_by_role("spinbutton")).to_have_value("2")
        await selected.get_by_role("spinbutton").fill("4")
        await page.locator(".grocery-preview summary").click()
        await expect(page.locator(".grocery-preview")).to_contain_text("4 cup")
        await expect(page.locator(".grocery-preview")).to_contain_text("3 cups")
        await page.screenshot(path=str(args.output / "create-desktop.png"), full_page=True)
        await page.get_by_role("button", name="Create list", exact=True).click()
        await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        recipe_url = page.url
        await expect(page.locator("h1")).to_have_text("This week’s groceries")
        await expect(page.locator(".grocery-row").filter(has_text="Garnish (optional)")).to_have_count(3)
        rice = page.locator(".grocery-row").filter(has_text="4 cup")
        await rice.locator('input[type="checkbox"]').click()
        await expect(rice.locator('input[type="checkbox"]')).to_be_checked()
        await expect(rice.get_by_role("button", name="Edit Rice", exact=True)).to_be_enabled()
        await rice.get_by_role("button", name="Recipe details for Rice", exact=True).click()
        await expect(page.locator(".grocery-inspector")).to_contain_text("E2E chickpea curry")
        await rice.get_by_role("button", name="Edit Rice", exact=True).click()
        dialog = page.get_by_role("dialog")
        await dialog.get_by_label("Ingredient / item name", exact=True).fill("Brown rice")
        await dialog.get_by_label("Amount", exact=True).fill("500")
        await dialog.get_by_label("Unit", exact=True).fill("g")
        await dialog.get_by_role("button", name="Save item", exact=True).click()
        await expect(page.locator(".grocery-row").filter(has_text="Brown rice")).to_be_visible()
        await expect(page.locator(".grocery-inspector")).to_contain_text("Edited on this list")
        await expect(page.locator(".grocery-inspector")).to_contain_text("500 g")
        await expect(page.locator(".grocery-inspector")).to_contain_text("Rice: 2 cup")
        await page.screenshot(path=str(args.output / "workspace-desktop.png"), full_page=True)
        # Source changes cannot alter the saved list or its attribution.
        sql(args.instance, f"DELETE FROM recipes WHERE id='{ids['own']}'; UPDATE recipes SET visibility='private' WHERE id='{ids['public']}';")
        await page.reload(wait_until="networkidle")
        await expect(page.locator(".grocery-row").filter(has_text="Brown rice").locator('input[type="checkbox"]')).to_be_checked()
        await page.locator(".grocery-row").filter(has_text="Brown rice").get_by_role("button", name="Recipe details for Brown rice").click()
        await expect(page.locator(".grocery-inspector")).to_contain_text("E2E chickpea curry")
        for width in (320, 390, 760, 1440):
            await page.set_viewport_size({"width": width, "height": 900})
            assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), f"Workspace overflow at {width}"
        await page.set_viewport_size({"width": 390, "height": 844})
        await page.screenshot(path=str(args.output / "workspace-phone.png"), full_page=True)
        await page.locator(".grocery-inspector").get_by_role("button", name="Close recipe details").click()
        await expect(page.get_by_role("button", name="Recipe details for Brown rice")).to_be_focused()
        await page.goto(base + "/grocery-lists/new", wait_until="networkidle")
        assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth")
        await page.screenshot(path=str(args.output / "create-phone.png"), full_page=True)
        await page.set_viewport_size({"width": 1440, "height": 1000})
        await page.goto(recipe_url, wait_until="networkidle")
        await page.get_by_role("button", name="Toggle theme", exact=True).click()
        await expect(page.locator("html")).to_have_class(re.compile("dark"))
        await page.screenshot(path=str(args.output / "workspace-dark.png"), full_page=True)
        await page.get_by_role("button", name="Rename", exact=True).click()
        await page.get_by_label("List name", exact=True).fill("Weekend groceries")
        await page.get_by_role("button", name="Save name", exact=True).click()
        await expect(page.locator("h1")).to_have_text("Weekend groceries")
        await page.locator(".grocery-list-link").filter(has_text="Household essentials").click()
        await expect(page.locator("h1")).to_have_text("Household essentials")
        await page.go_back()
        await expect(page.locator("h1")).to_have_text("Weekend groceries")
        await page.get_by_role("button", name="Delete list", exact=True).click()
        await page.get_by_role("dialog").get_by_role("button", name="Delete list", exact=True).click()
        await expect(page.locator("h1")).to_have_text("Household essentials")
        anonymous = await browser.new_context()
        anonymous_page = await anonymous.new_page()
        await anonymous_page.goto(manual_url, wait_until="networkidle")
        assert "Dish soap" not in await anonymous_page.locator("body").inner_text()
        await anonymous.close()
        assert not errors, errors
        result = {"passed": True, "browser": browser.version,
                  "checks": ["manual creation", "network failure rollback", "database persistence across sessions",
                             "own/public/unlisted recipe selection", "inaccessible private source rejection",
                             "serving validation and exact-unit aggregation", "recipe reselection resets serving controls", "all ingredients copied",
                             "full name/amount/unit editing", "original provenance retained", "source deletion independence",
                             "320/390/760/1440px layout", "inspector focus restoration", "light/dark themes",
                             "rename", "list switching and browser back", "delete active list", "anonymous isolation"],
                  "page_errors": errors}
        (args.output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
        await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", default="noms-groceries")
    parser.add_argument("--output", type=Path, default=Path("/tmp/grocery-e2e"))
    asyncio.run(main(parser.parse_args()))
