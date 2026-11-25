#!/usr/bin/env python3
"""Exercise the real GLB drawer, its loading fallback, and its reopen lifecycle."""
import argparse
from urllib.parse import urlsplit

from playwright.sync_api import sync_playwright


def check(base_url):
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(
            executable_path="/usr/bin/chromium",
            args=["--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
        )
        page = browser.new_page(viewport={"width": 1440, "height": 1000})
        page.goto(base_url.rstrip("/") + "/login", wait_until="networkidle")
        trigger = page.get_by_role("button", name="Open navigation menu")
        slots = page.locator(".drawer-model-icon")
        drawn = page.locator(".drawer-model-icon[data-ready]")
        assert slots.count() == 0

        # Make one model fail. Its rendered poster and destination must still work.
        page.route("**/home.glb", lambda route: route.abort())
        trigger.click()
        page.wait_for_function("document.querySelectorAll('.drawer-model-icon[data-ready]').length === 9")
        home = page.locator('.drawer-model-icon[data-icon="home"]')
        assert home.locator("img").evaluate("img => img.complete && img.naturalWidth > 0")
        assert not home.locator("canvas").is_visible()
        page.keyboard.press("Escape")
        page.wait_for_selector("noms-drawer-models", state="detached")
        page.unroute("**/home.glb")

        trigger.click()
        page.wait_for_function("document.querySelectorAll('.drawer-model-icon[data-ready]').length === 10")
        assert drawn.count() == slots.count()
        for canvas in drawn.locator("canvas").all():
            assert canvas.evaluate("""c => c.getContext('2d')
                .getImageData(0, 0, c.width, c.height).data.some((v, i) => i % 4 === 3 && v > 64)""")
        nav = page.get_by_role("navigation", name="All destinations")
        kitchen = page.locator('[data-icon="kitchen"] canvas')
        resting = kitchen.evaluate("c => c.toDataURL()")
        nav.get_by_role("link", name="Kitchen").hover()
        page.wait_for_function(
            "frame => document.querySelector('[data-icon=kitchen] canvas').toDataURL() !== frame", arg=resting)
        nav.get_by_role("link", name="Explore").click()
        page.wait_for_url("**/explore")
        assert slots.count() == 0
        browser.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url", help="Local Noms app origin")
    args = parser.parse_args()
    if urlsplit(args.url).hostname not in {"localhost", "127.0.0.1", "::1"}:
        parser.error("Expected a local Noms app")
    check(args.url)
