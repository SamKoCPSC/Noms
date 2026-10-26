#!/usr/bin/env python3
"""Kitchen navigation regression checks on a local mock-auth development stack."""
import argparse
import asyncio
import json
import re
import uuid
from pathlib import Path
from urllib.parse import urlsplit


async def main(args):
    from playwright.async_api import async_playwright, expect

    assert re.fullmatch(r"noms-[a-z0-9][a-z0-9-]*", args.instance)
    metadata = dict(line.split("=", 1) for line in
                    (Path("/tmp/noms-dev") / args.instance / "metadata").read_text().splitlines() if "=" in line)
    base = metadata["APP_URL"]
    assert metadata["STATE"] == "ready" and urlsplit(base).hostname in {"127.0.0.1", "localhost", "::1"}
    args.output.mkdir(parents=True, exist_ok=True)
    checks, errors = [], []
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])
        context = await browser.new_context(viewport={"width": 1440, "height": 1100})
        page = await context.new_page()
        page.set_default_timeout(20000)
        page.set_default_navigation_timeout(120000)
        page.on("pageerror", lambda error: errors.append(str(error)))
        await page.goto(base + "/login", wait_until="networkidle")
        await page.get_by_role("link", name="Continue with Google", exact=True).click()
        assert await page.locator("input[name=username]").count(), "Expected local mock OAuth"
        suffix = uuid.uuid4().hex[:12]
        await page.locator("input[name=username]").fill("kitchen-" + suffix)
        await page.locator("textarea[name=claims]").fill(json.dumps({
            "email": f"kitchen-{suffix}@example.test", "email_verified": True, "name": "Kitchen Demo"}))
        await page.locator("input[type=submit]").click()
        await page.wait_for_url(base + "/**")
        await page.goto(base + "/kitchen", wait_until="networkidle")
        views = page.get_by_role("navigation", name="Kitchen views", exact=True)
        await expect(views.get_by_role("link", name="Pantry", exact=True)).to_have_attribute("aria-current", "page")
        await expect(page.locator(".navbar-links").get_by_role("link", name="Kitchen", exact=True)).to_have_count(1)
        await expect(page.locator(".navbar-links").get_by_role("link", name="Pantry", exact=True)).to_have_count(0)
        await expect(page.locator(".navbar-links").get_by_role("link", name="Grocery lists", exact=True)).to_have_count(0)
        await expect(page.locator(".navbar-links").get_by_role("link", name="Kitchen", exact=True)).to_have_attribute("aria-current", "page")
        checks.append("one Kitchen navigation destination and default Pantry view")
        original_url = page.url
        async with context.expect_page() as opened:
            await views.get_by_role("link", name="Pantry", exact=True).click(modifiers=["Control"])
        separate = await opened.value
        await separate.wait_for_load_state("networkidle")
        await expect(separate.locator(".kitchen-title")).to_have_text("Kitchen")
        assert page.url == original_url
        await separate.close()
        checks.append("view links preserve native Ctrl-click navigation in another tab")

        # Create two independent pantries using both the old and canonical create routes.
        pantry_urls = []
        for path, name in [("/pantry/new", "Home"), ("/kitchen/pantry/new", "Cabin")]:
            await page.goto(base + path, wait_until="networkidle")
            await expect(views.get_by_role("link", name="Pantry", exact=True)).to_have_attribute("aria-current", "page")
            await page.get_by_label("Pantry name", exact=True).fill(name)
            await page.get_by_role("button", name="Create pantry", exact=True).click()
            await page.wait_for_url(re.compile(r".*/kitchen/pantry/[0-9a-f-]{36}$"))
            pantry_urls.append(page.url)
        checks.append("renamed pantry creation and legacy create bookmark")

        await views.get_by_role("link", name="Grocery lists", exact=True).click()
        await expect(page.locator(".kitchen-title")).to_have_text("Kitchen")
        grocery_urls = []
        for name in ["This week", "Household essentials"]:
            await page.goto(base + "/kitchen/grocery-lists/new", wait_until="networkidle")
            await expect(views.get_by_role("link", name="Grocery lists", exact=True)).to_have_attribute("aria-current", "page")
            await page.get_by_label("List name", exact=True).fill(name)
            await page.get_by_role("button", name="Create list", exact=True).click()
            await page.wait_for_url(re.compile(r".*/kitchen/grocery-lists/[0-9a-f-]{36}$"))
            grocery_urls.append(page.url)
        await page.locator(".grocery-list-links").get_by_role("link").filter(has_text="This week").click()
        await expect(page.locator("h1")).to_have_text("This week")
        await views.get_by_role("link", name="Pantry", exact=True).click()
        await expect(page.locator("h1")).to_have_text("Cabin")
        await page.locator(".pantry-profile-link").filter(has_text="Home").click()
        await expect(page.locator("h1")).to_have_text("Home")
        await views.get_by_role("link", name="Grocery lists", exact=True).click()
        await expect(page.locator("h1")).to_have_text("This week")
        await views.get_by_role("link", name="Pantry", exact=True).click()
        await expect(page.locator("h1")).to_have_text("Home")
        await page.go_back()
        await expect(page.locator("h1")).to_have_text("This week")
        await expect(views.get_by_role("link", name="Grocery lists", exact=True)).to_have_attribute("aria-current", "page")
        checks.append("multiple independent lists and pantries; view switching retains selected records; browser Back")

        # Old detail bookmarks still open the same database rows and show the shared shell.
        for canonical, title in [(pantry_urls[0], "Home"), (grocery_urls[0], "This week")]:
            await page.goto(canonical.replace("/kitchen/", "/"), wait_until="networkidle")
            await expect(page.locator("h1")).to_have_text(title)
            await expect(views).to_be_visible()
        for path, view in [("/pantry", "Pantry"), ("/grocery-lists", "Grocery lists"), ("/grocery-lists/new", "Grocery lists")]:
            await page.goto(base + path, wait_until="networkidle")
            await expect(views.get_by_role("link", name=view, exact=True)).to_have_attribute("aria-current", "page")
        checks.append("all legacy root, create, and detail bookmarks retain workspace context")

        await page.goto(pantry_urls[0], wait_until="networkidle")
        await page.screenshot(path=str(args.output / "pantry-desktop.png"), full_page=True)
        await views.get_by_role("link", name="Grocery lists", exact=True).click()
        await page.screenshot(path=str(args.output / "groceries-desktop.png"), full_page=True)
        for path in [pantry_urls[0], grocery_urls[0], base + "/kitchen/pantry/new", base + "/kitchen/grocery-lists/new"]:
            await page.goto(path, wait_until="networkidle")
            for width in [320, 390, 760, 1100, 1440]:
                await page.set_viewport_size({"width": width, "height": 950})
                assert await page.evaluate("document.documentElement.scrollWidth <= innerWidth"), (path, width)
                assert await views.get_by_role("link", name="Pantry", exact=True).evaluate("e => e.getBoundingClientRect().height >= 44")
                assert await views.get_by_role("link", name="Grocery lists", exact=True).evaluate("e => e.getBoundingClientRect().height >= 44")
        checks.append("both workspaces and create forms fit 320/390/760/1100/1440px with accessible view links")

        await page.set_viewport_size({"width": 390, "height": 844})
        await page.goto(pantry_urls[0], wait_until="networkidle")
        mobile = page.get_by_role("navigation", name="Mobile navigation", exact=True)
        await expect(mobile.get_by_role("link", name="Kitchen", exact=True)).to_have_attribute("aria-current", "page")
        await page.screenshot(path=str(args.output / "pantry-phone.png"), full_page=True)
        await page.get_by_role("button", name="Open navigation menu", exact=True).click()
        drawer = page.get_by_role("dialog")
        await expect(drawer.get_by_role("link", name="Kitchen", exact=True)).to_have_count(1)
        await expect(drawer.get_by_role("link", name="Pantry", exact=True)).to_have_count(0)
        await drawer.get_by_role("button", name=re.compile("Dark Mode")).click()
        await page.get_by_role("button", name="Close navigation menu", exact=True).click()
        await expect(page.locator("html")).to_have_class(re.compile("dark"))
        await page.screenshot(path=str(args.output / "pantry-dark-phone.png"), full_page=True)
        await views.get_by_role("link", name="Grocery lists", exact=True).click()
        await expect(mobile.get_by_role("link", name="Kitchen", exact=True)).to_have_attribute("aria-current", "page")
        checks.append("mobile Kitchen tab, unified drawer, and dark theme")

        anonymous = await browser.new_context()
        guest = await anonymous.new_page()
        for url in [pantry_urls[0], grocery_urls[0]]:
            await guest.goto(url, wait_until="networkidle")
            await expect(guest.get_by_text("Sign in to continue", exact=True)).to_be_visible()
            await expect(guest.get_by_role("navigation", name="Kitchen views", exact=True)).to_have_count(0)
        await anonymous.close()
        checks.append("anonymous visitors cannot access either nested private view")
        assert not errors, errors
        result = {"passed": True, "browser": browser.version, "checks": checks, "page_errors": errors}
        (args.output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
        await browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instance", default="noms-groceries")
    parser.add_argument("--output", type=Path, default=Path("/workspace/noms-kitchen-e2e"))
    asyncio.run(main(parser.parse_args()))
