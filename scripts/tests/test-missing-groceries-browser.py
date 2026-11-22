#!/usr/bin/env python3
"""Missing-ingredient grocery creation against the disposable local mock-auth stack."""
import argparse
import asyncio
import importlib.util
import json
import re
import uuid
from pathlib import Path
from urllib.parse import urlsplit

spec = importlib.util.spec_from_file_location("groceries", Path(__file__).with_name("test-grocery-list-browser.py"))
helpers = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helpers)


async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=30000)
    assert re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance)
    meta = dict(line.split("=", 1) for line in (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
    base = meta["APP_URL"]
    assert meta["STATE"] == "ready" and urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    args.output.mkdir(parents=True, exist_ok=True)
    errors, checks, paths = [], [], {}

    def passed(label):
        checks.append(label)
        print("PASS:", label, flush=True)

    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])
        ctx = await browser.new_context(viewport={"width": 1440, "height": 1100})
        await ctx.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
        page = await ctx.new_page()
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.set_default_navigation_timeout(120000)
        suffix = uuid.uuid4().hex[:10]
        await page.goto(base + "/login", wait_until="networkidle")
        await page.get_by_role("link", name="Continue with Google").click()
        assert urlsplit(page.url).hostname == "127.0.0.1" and str(urlsplit(page.url).port) == meta["MOCK_OAUTH_PORT"]
        await page.locator("input[name=username]").fill("missing-" + suffix)
        await page.locator("textarea[name=claims]").fill(json.dumps({"email": f"missing-{suffix}@example.test", "email_verified": True, "name": "Grocery Tester " + suffix}))
        await page.locator("input[type=submit]").click()
        await page.wait_for_url(base + "/**")
        owner = str(uuid.UUID((await (await ctx.request.get(base + "/api/user_profile")).json())["current_user_id"]))
        await page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
        await page.get_by_role("button", name="From recipes", exact=False).click()
        await page.get_by_label("Only missing ingredients", exact=True).check()
        await expect(page.locator(".grocery-ingredient-options")).to_contain_text("Create a pantry first")
        await expect(page.get_by_role("button", name="Create list", exact=True)).to_be_disabled()
        passed("No-pantry guidance and disabled missing-only save")
        pantry, recipe, second, other, foreign = [str(uuid.uuid4()) for _ in range(5)]
        ingredients = json.dumps([
            {"name": "Rice", "amount": "100", "unit": "g"},
            {"name": "Beans", "amount": "200", "unit": "g"},
            {"name": "Parsley", "amount": "5", "unit": "g", "optional": True},
            {"name": "Beans", "amount": "10", "unit": "g", "optional": True},
        ])
        def sql(statement):
            try:
                return helpers.sql(args.instance, statement)
            except helpers.subprocess.CalledProcessError as error:
                raise RuntimeError(error.stderr) from error
        literal = helpers.literal
        sql(f"""
INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash) VALUES('{pantry}','{owner}','Home pantry','{uuid.uuid4()}','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
INSERT INTO pantry_ingredients(profile_id,name,amount,unit,quantity_zero,available,position) VALUES('{pantry}','RICE','0','ml',TRUE,TRUE,0),('{pantry}','Beans','999','g',FALSE,FALSE,1);
INSERT INTO recipes(id,user_id,title,servings,ingredients,visibility) VALUES('{recipe}','{owner}','Bean bowls',2,{literal(ingredients)}::jsonb,'private'),('{second}','{owner}','More beans',2,'[{{"name":"Beans","amount":"50","unit":"g"}}]'::jsonb,'private');
INSERT INTO users(id,username,display_name,email) VALUES('{other}','other_{suffix}','Other {suffix}','other-{suffix}@example.test');
INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash) VALUES('{foreign}','{other}','Private pantry','{uuid.uuid4()}','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
""")
        def record(request):
            path = urlsplit(request.url).path
            for name in ["create_grocery_list", "get_kitchen"]:
                if path.startswith("/api/" + name):
                    paths[name] = path
        page.on("request", record)
        route = base + f"/kitchen/grocery-lists/new/from-pantry/{pantry}/{recipe}"
        await page.goto(base + f"/recipes/{recipe}", wait_until="networkidle")
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await page.get_by_role("link", name="Create list from missing ingredients", exact=True).click()
        await expect(page.get_by_label("Only missing ingredients", exact=True)).to_be_checked()
        optional = page.get_by_label("Include optional ingredients", exact=True)
        await expect(optional).to_be_checked()
        await expect(page.get_by_label("Compare with pantry", exact=True)).to_have_value(pantry)
        review = page.locator(".grocery-preview")
        await review.locator("summary").click()
        await expect(review.locator(".grocery-draft-row")).to_have_count(2)
        await expect(review).to_contain_text("210 g")
        await expect(review).not_to_contain_text("Rice")
        await optional.uncheck()
        await expect(review.locator(".grocery-draft-row")).to_have_count(1)
        await expect(review).to_contain_text("200 g")
        await optional.check()
        await page.locator(".grocery-selected-recipe").get_by_label("Servings", exact=True).fill("4")
        await expect(review).to_contain_text("420 g")
        await expect(review).to_contain_text("10 g")
        passed("Recipe pantry link, default optional inclusion, exclusion and serving scaling")
        for width in [390, 1440]:
            await page.set_viewport_size({"width": width, "height": 1000})
            await page.locator(".grocery-ingredient-options").scroll_into_view_if_needed()
            assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth")
            await expect(page.locator('.kitchen-view[aria-current="page"]')).to_have_text("Grocery lists")
            if width == 390:
                await expect(page.locator('.mobile-bottom-nav')).not_to_be_visible()
            await page.screenshot(path=str(args.output / f"missing-groceries-{width}.png"))
        passed("Mobile and desktop layout without horizontal overflow")
        await page.get_by_label("List name", exact=True).fill("Missing shop")
        form = page.locator(".grocery-add-row")
        await form.get_by_label("Item name", exact=True).fill("Rice")
        await form.get_by_label("Amount", exact=True).fill("20")
        await form.get_by_label("Unit", exact=True).fill("g")
        # Server detects an inventory change rather than silently saving a different preview.
        sql(f"UPDATE kitchen_profiles SET revision=revision+1 WHERE id='{pantry}';")
        await page.get_by_role("button", name="Create list", exact=True).click()
        await expect(page.locator(".grocery-error").first).to_contain_text("selected pantry changed")
        await expect(page.get_by_role("button", name="Create list", exact=True)).to_be_enabled()
        assert sql(f"SELECT count(*) FROM grocery_lists WHERE owner_id='{owner}';") == "0"
        passed("Stale pantry rejected atomically and preview reloaded for review")
        # Lose response AFTER successful server save, then retry the frozen request.
        async def lost_response(route):
            await route.fetch()
            await route.abort()
        await page.route("**/api/create_grocery_list*", lost_response, times=1)
        await page.get_by_role("button", name="Create list", exact=True).click()
        await expect(page.get_by_role("button", name="Retry creation", exact=True)).to_be_enabled()
        await page.get_by_role("button", name="Retry creation", exact=True).click()
        await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        list_url = page.url
        await expect(page.locator(".grocery-row")).to_have_count(3)
        await page.reload(wait_until="networkidle")
        await expect(page.locator(".grocery-row")).to_have_count(3)
        assert sql(f"SELECT count(*) FROM grocery_lists WHERE owner_id='{owner}';") == "1"
        passed("Saved list persistence, manual extras and safe recovery after a lost response")
        # Optional exclusion + multi-recipe merging via the ordinary creation form.
        await page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
        await page.get_by_role("button", name="From recipes", exact=False).click()
        await page.get_by_label("Only missing ingredients", exact=True).check()
        await page.get_by_label("Include optional ingredients", exact=True).uncheck()
        for title in ["Bean bowls", "More beans"]:
            await page.locator(".grocery-recipe-option").filter(has=page.get_by_text(title, exact=True)).get_by_role("button", name="Add", exact=True).click()
        await page.get_by_label("List name", exact=True).fill("Required shop")
        await page.locator(".grocery-preview summary").click()
        await expect(page.locator(".grocery-preview")).to_contain_text("250 g")
        await expect(page.locator(".grocery-preview .grocery-draft-row")).to_have_count(1)
        await page.get_by_role("button", name="Create list", exact=True).click()
        await page.wait_for_url(re.compile(r".*/grocery-lists/[0-9a-f-]{36}$"))
        await expect(page.locator(".grocery-row")).to_have_count(1)
        passed("Kitchen creation, optional exclusion and multi-recipe aggregation")
        # Request references never grant access to another account's pantry.
        async def post(context, payload):
            return await context.request.post(base + paths["create_grocery_list"], data=json.dumps(payload), headers={"Content-Type": "application/json", "Origin": base})
        payload = {"expected_account_id": owner, "request": {"request_id": str(uuid.uuid4()), "name": "Forbidden", "note": "", "recipes": [{"recipe_id": recipe, "target_servings": None, "expected_revision": "a"*64}], "manual_items": [], "pantry": {"id": foreign, "expected_revision": 0}, "include_optional": True}}
        response = await post(ctx, payload)
        assert response.status >= 400 and "Pantry not accessible" in await response.text()
        anonymous = await browser.new_context()
        assert (await post(anonymous, payload)).status >= 400
        await anonymous.close()
        passed("Foreign pantry and anonymous creation rejected")
        # With everything stocked there is an explicit empty-preview message, but extras remain possible.
        sql(f"UPDATE pantry_ingredients SET available=TRUE WHERE profile_id='{pantry}'; INSERT INTO pantry_ingredients(profile_id,name,amount,unit,available,position) VALUES('{pantry}','Parsley','','g',TRUE,2); UPDATE kitchen_profiles SET revision=revision+1 WHERE id='{pantry}';")
        await page.goto(route, wait_until="networkidle")
        await page.locator(".grocery-preview summary").click()
        await expect(page.locator(".grocery-preview")).to_contain_text("No ingredients to add")
        await page.get_by_label("Only missing ingredients", exact=True).uncheck()
        await expect(page.locator(".grocery-preview .grocery-draft-row")).to_have_count(3)
        await page.goto(list_url, wait_until="networkidle")
        await expect(page.locator(".grocery-row")).to_have_count(3)
        passed("All-stocked feedback, full-list fallback and saved-list independence")
        assert not errors, errors
        (args.output / "result.json").write_text(json.dumps({"passed": checks, "console_errors": errors}, indent=2))
        await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--instance", default="noms-groceries")
    parser.add_argument("--output", type=Path, default=Path("/tmp/noms-missing-groceries-e2e"))
    asyncio.run(main(parser.parse_args()))
