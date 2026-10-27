#!/usr/bin/env python3
"""Pantry cookable recipes against a ready, loopback, synthetic-auth dev stack."""
import argparse
import asyncio
import json
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import urlsplit


def sql(instance, statement):
    return subprocess.run(["docker", "exec", "-i", f"{instance}-postgres-1", "sh", "-c",
                           'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -At'],
                          input=statement, capture_output=True, text=True, check=True).stdout.strip()


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=20000)
    assert re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance)
    metadata = dict(line.split("=", 1) for line in
                    (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
    base = metadata["APP_URL"]
    assert metadata["STATE"] == "ready" and urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    args.output.mkdir(parents=True, exist_ok=True)
    checks, errors, paths = [], [], {}

    def passed(label):
        checks.append(label)
        print("PASS:", label, flush=True)

    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])

        async def context():
            ctx = await browser.new_context(viewport={"width": 1440, "height": 1100})
            await ctx.route(re.compile(r"https://fonts\.(googleapis|gstatic)\.com/"), lambda route: route.abort())
            ctx.on("page", lambda page: page.on("pageerror", lambda error: errors.append(str(error))))
            return ctx

        async def login(ctx):
            page = await ctx.new_page()
            page.set_default_navigation_timeout(120000)
            suffix = uuid.uuid4().hex[:10]
            await page.goto(base + "/login", wait_until="networkidle")
            await page.get_by_role("link", name="Continue with Google").click()
            origin = urlsplit(page.url)
            assert origin.hostname == "127.0.0.1" and str(origin.port) == metadata["MOCK_OAUTH_PORT"]
            await page.locator("input[name=username]").fill("pantry-cook-" + suffix)
            await page.locator("textarea[name=claims]").fill(json.dumps({"email": f"pantry-cook-{suffix}@example.test", "email_verified": True, "name": "Pantry Recipes Demo"}))
            await page.locator("input[type=submit]").click()
            await page.wait_for_url(base + "/**")
            owner = str(uuid.UUID((await (await ctx.request.get(base + "/api/user_profile")).json())["current_user_id"]))
            return page, owner

        ctx = await context()
        page, owner = await login(ctx)

        def record(request):
            path = urlsplit(request.url).path
            for name in ["find_pantry_recipes", "get_kitchen"]:
                if path.startswith("/api/" + name):
                    paths[name] = path
        page.on("request", record)

        async def create_pantry(name, ingredients):
            await page.goto(base + "/kitchen/pantry/new", wait_until="networkidle")
            await page.get_by_label("Pantry name", exact=True).fill(name)
            form = page.locator("form.pantry-add")
            for label, amount, unit in ingredients:
                await form.get_by_label("Ingredient name", exact=True).fill(label)
                await form.get_by_label("Amount", exact=True).fill(amount)
                await form.get_by_label("Unit", exact=True).fill(unit)
                await form.get_by_role("button", name="Add ingredient", exact=True).click()
            await page.get_by_role("button", name="Create pantry", exact=True).click()
            await page.wait_for_url(re.compile(r".*/kitchen/pantry/[0-9a-f-]{36}$"))
            await expect(page.get_by_role("heading", name="What can I cook?", exact=True)).to_be_visible()
            return page.url, str(uuid.UUID(page.url.rsplit("/", 1)[1]))

        home, home_id = await create_pantry("Home pantry", [("Rice", "0", "kg"), ("Salt", "", "")])
        section = page.locator(".pantry-cookable")
        await expect(section).to_contain_text("No matching recipes yet")
        cabin, cabin_id = await create_pantry("Cabin pantry", [("rice", "1", "cup")])
        other = str(uuid.uuid4())
        root, child, outsider_collection = [str(uuid.uuid4()) for _ in range(3)]
        sql(args.instance, f"""
INSERT INTO users(id,username,display_name,email) VALUES('{other}','other_{other[:8]}','Other cook','{other}@example.test');
INSERT INTO collections(id,owner_id,name) VALUES('{root}','{owner}','Favourites'),('{outsider_collection}','{other}','Other collection');
INSERT INTO collections(id,owner_id,name,parent_collection_id) VALUES('{child}','{owner}','Weeknight meals','{root}');
INSERT INTO pantry_equipment(profile_id,name,position,available) VALUES('{home_id}','Pot',0,TRUE);
""")
        ids = {}
        definitions = [
            ("Owned rice", owner, "private", ["RICE"]),
            ("Owned supper", owner, "private", ["rice", "SALT"]),
            ("Saved public", other, "public", ["Rice"]),
            ("Saved unlisted", other, "unlisted", ["Salt"]),
            ("Duplicate favourite", owner, "unlisted", ["rice", "RICE"]),
            ("Unsaved public", other, "public", ["Rice"]),
            ("Unsaved unlisted", other, "unlisted", ["Salt"]),
            ("Hidden private", other, "private", ["Rice"]),
            ("Other collection", other, "public", ["Rice"]),
            ("Needs milk", owner, "private", ["Milk"]),
            ("No ingredients", owner, "private", []),
            ("Only equipment", owner, "private", ["Pot"]),
            ("Optional prose", owner, "private", ["Garnish (optional)"]),
        ]
        for title, actor, visibility, names in definitions:
            recipe_id = str(uuid.uuid4())
            ids[title] = recipe_id
            ingredients = json.dumps([{"name": name, "amount": "1000", "unit": "cup"} for name in names])
            sql(args.instance, f"INSERT INTO recipes(id,user_id,title,visibility,ingredients,prep_time_minutes,cook_time_minutes,equipment) VALUES('{recipe_id}','{actor}',{literal(title)},{literal(visibility)},{literal(ingredients)}::jsonb,10,15,'[{{\"name\":\"Oven\"}}]');")
        sql(args.instance, f"""
INSERT INTO collection_recipes(collection_id,recipe_id) VALUES
('{root}','{ids['Saved public']}'),('{child}','{ids['Saved public']}'),
('{child}','{ids['Saved unlisted']}'),('{root}','{ids['Duplicate favourite']}'),
('{child}','{ids['Duplicate favourite']}'),('{root}','{ids['Hidden private']}'),
('{outsider_collection}','{ids['Other collection']}');
INSERT INTO recipe_drafts(user_id,snapshot) VALUES('{owner}','{{"title":"Unpublished draft","ingredients":[{{"name":"Rice","amount":"1","unit":"cup"}}]}}');
""")
        await page.goto(home, wait_until="networkidle")
        await expect(section.locator(".recipe-card")).to_have_count(5)
        expected = {"Owned rice", "Owned supper", "Saved public", "Saved unlisted", "Duplicate favourite"}
        assert set(await section.locator("h3").all_text_contents()) == expected
        await expect(section).to_contain_text("Quantities and equipment aren’t checked")
        await expect(section).to_contain_text("25m")
        inventory_bottom = await page.locator(".grocery-detail-layout").evaluate("el => el.getBoundingClientRect().bottom")
        assert await section.evaluate("el => el.getBoundingClientRect().top") >= inventory_bottom
        passed("section below inventory: owned plus saved/nested recipes, deduplicated; private/unsaved/draft/empty/equipment exclusions")

        async def post(ctx, name, payload):
            return await ctx.request.post(base + paths[name], data=json.dumps(payload), headers={"Content-Type": "application/json"})

        response = await post(ctx, "get_kitchen", {"expected_account_id": owner, "id": home_id})
        revision = (await response.json())["revision"]
        body = {"expected_account_id": owner, "pantry_id": home_id, "pantry_revision": revision, "cursor": None}
        response = await post(ctx, "find_pantry_recipes", body)
        assert response.ok
        payload = await response.json()
        assert len(payload["recipes"]) == 5 and all("ingredients" not in r and "instructions" not in r for r in payload["recipes"])
        passed("real authenticated endpoint returns bounded cards; case variants and recorded zero/unknown amounts match")

        for width in [320, 390, 760, 1100, 1440]:
            await page.set_viewport_size({"width": width, "height": 1100})
            assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), width
            await section.evaluate("el => {el.style.scrollMarginTop='110px';el.scrollIntoView({block:'start',behavior:'instant'});}")
            if width in [390, 1440]:
                await section.screenshot(path=str(args.output / f"recipes-{width}.png"))
        await page.get_by_role("button", name="Toggle theme", exact=True).click()
        await section.screenshot(path=str(args.output / "recipes-dark.png"))
        await page.get_by_role("button", name="Toggle theme", exact=True).click()
        passed("five responsive widths and light/dark recipe cards, with cooking time and detail links")

        await section.locator(".recipe-card").filter(has_text="Owned rice").get_by_role("link").first.click()
        await page.wait_for_url(base + "/recipes/" + ids["Owned rice"])
        await expect(page.get_by_role("heading", name="Owned rice", exact=True)).to_be_visible()
        await page.get_by_role("button", name="Check pantry", exact=True).click()
        await page.get_by_label("Pantry", exact=True).select_option(home_id)
        await expect(page.locator(".recipe-pantry__verdict")).to_have_text("All ingredients available")
        await page.goto(home, wait_until="networkidle")
        passed("recipe cards open live details and agree with the existing pantry presence check")

        started, release = asyncio.Event(), asyncio.Event()
        async def hold(route):
            started.set()
            await release.wait()
            await route.continue_()
        await page.route("**/api/find_pantry_recipes*", hold)
        await page.locator(".pantry-profile-link").filter(has_text="Cabin pantry").click()
        await asyncio.wait_for(started.wait(), timeout=20)
        try:
            await expect(section.locator(".recipe-card")).to_have_count(0)
            await expect(section).to_contain_text("Finding recipes")
        finally:
            release.set()
        await expect(section.locator(".recipe-card")).to_have_count(3)
        await page.unroute("**/api/find_pantry_recipes*", hold)
        assert set(await section.locator("h3").all_text_contents()) == {"Owned rice", "Saved public", "Duplicate favourite"}
        await page.locator(".pantry-profile-link").filter(has_text="Home pantry").click()
        await expect(section.locator(".recipe-card")).to_have_count(5)
        await page.get_by_role("button", name="Mark Rice out", exact=True).click()
        await expect(section.locator(".recipe-card")).to_have_count(1)
        await expect(section.locator("h3")).to_have_text("Saved unlisted")
        await page.get_by_role("button", name="Restore Rice", exact=True).click()
        await expect(section.locator(".recipe-card")).to_have_count(5)
        passed("pantry switching hides old results while pending; availability edits automatically recheck recipes")

        await page.route("**/api/find_pantry_recipes*", lambda route: route.abort())
        await page.reload(wait_until="networkidle")
        await expect(section.get_by_role("button", name="Retry recipe check")).to_be_visible()
        await expect(section.locator(".recipe-card")).to_have_count(0)
        await page.unroute("**/api/find_pantry_recipes*")
        await section.get_by_role("button", name="Retry recipe check").click()
        await expect(section.locator(".recipe-card")).to_have_count(5)
        sql(args.instance, f"UPDATE recipes SET visibility='private' WHERE id='{ids['Saved public']}';")
        await page.reload(wait_until="networkidle")
        await expect(section.locator(".recipe-card")).to_have_count(4)
        await expect(section).not_to_contain_text("Saved public")
        sql(args.instance, f"UPDATE recipes SET visibility='public' WHERE id='{ids['Saved public']}';")
        passed("failed reads hide results and recover; changed source visibility is honored on fresh reads")

        for i in range(17):
            sql(args.instance, f"INSERT INTO recipes(user_id,title,ingredients) VALUES('{owner}','Ready page {i:02}','[{{\"name\":\"Rice\",\"amount\":\"1\",\"unit\":\"\"}}]');")
        # Exercise a meaningful library scan without returning large recipe bodies.
        sql(args.instance, f"INSERT INTO recipes(user_id,title,ingredients) SELECT '{owner}'::uuid,'Unmatched fixture '||n,'[{{\"name\":\"Milk\",\"amount\":\"1\",\"unit\":\"\"}}]'::jsonb FROM generate_series(1,300) n;")
        await page.reload(wait_until="networkidle")
        await expect(section.locator(".recipe-card")).to_have_count(12)
        first = await section.locator("h3").all_text_contents()
        await section.get_by_role("button", name="Next recipes", exact=True).click()
        await expect(section.locator(".recipe-card")).to_have_count(10)
        second = await section.locator("h3").all_text_contents()
        assert len(set(first + second)) == 22 and expected <= set(first + second)
        await expect(section.get_by_role("button", name="Next recipes", exact=True)).to_have_count(0)
        await section.get_by_role("button", name="Previous recipes", exact=True).click()
        await expect(section.locator(".recipe-card")).to_have_count(12)
        assert await section.locator("h3").all_text_contents() == first
        source = Path("src/db/pantry_recipes.rs").read_text()
        query = re.search(r'CANDIDATES: &str\s*=\s*"(.*?)";', source, re.S).group(1)
        plan = sql(args.instance, f"PREPARE pantry_recipe_plan(uuid,timestamptz,uuid,bigint) AS {query}; EXPLAIN (ANALYZE,BUFFERS,FORMAT JSON) EXECUTE pantry_recipe_plan('{owner}',NULL,NULL,1048576);")
        (args.output / "query-plan.txt").write_text(plan + "\n")
        passed("12-card cursor pagination has no duplicates or omissions; query plan captured for a 300-unmatched-recipe library")

        latest = await post(ctx, "get_kitchen", {"expected_account_id": owner, "id": home_id})
        body["pantry_revision"] = (await latest.json())["revision"]
        stranger = await context()
        stranger_page, stranger_id = await login(stranger)
        response = await post(stranger, "find_pantry_recipes", {**body, "expected_account_id": stranger_id})
        assert response.status >= 400
        response = await post(ctx, "find_pantry_recipes", {**body, "expected_account_id": stranger_id})
        assert response.status >= 400
        response = await post(ctx, "find_pantry_recipes", {**body, "cursor": "bad!"})
        assert response.status >= 400
        response = await post(ctx, "find_pantry_recipes", {**body, "pantry_revision": -1})
        assert response.status >= 400
        anonymous = await context()
        response = await post(anonymous, "find_pantry_recipes", {**body, "expected_account_id": None})
        assert response.status >= 400
        anon_page = await anonymous.new_page()
        await anon_page.goto(home, wait_until="networkidle")
        await expect(anon_page.locator(".pantry-cookable")).to_have_count(0)
        passed("cross-owner API access, spoofed account binding, invalid cursor/revision and anonymous access are rejected")
        await anonymous.close()
        await stranger.close()
        assert not errors, errors
        result = {"passed": len(checks), "checks": checks, "page_errors": errors, "browser": browser.version}
        (args.output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
        await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", default="noms-groceries")
    parser.add_argument("--output", type=Path, default=Path("/workspace/noms-pantry-recipes-e2e"))
    asyncio.run(main(parser.parse_args()))
