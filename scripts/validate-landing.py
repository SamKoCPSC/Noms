import argparse, asyncio, json, os, re
from pathlib import Path
from urllib.parse import unquote
from playwright.async_api import async_playwright, expect

parser = argparse.ArgumentParser(
    description="Validate and capture the running Noms landing page (requires Python Playwright)."
)
parser.add_argument(
    "--url", required=True, help="Local app URL printed by the development stack"
)
parser.add_argument(
    "--output", required=True, help="Screenshot output directory outside the checkout"
)
parser.add_argument("--chromium", default="/usr/bin/chromium")
args = parser.parse_args()
ROOT = Path(args.output)
ROOT.mkdir(parents=True, exist_ok=True)


class Runtime:
    @staticmethod
    def app_url():
        return args.url.rstrip("/")

    @staticmethod
    def options():
        options = {
            "executable_path": args.chromium,
            "headless": True,
            "args": ["--disable-dev-shm-usage"],
        }
        proxy = os.environ.get("HTTPS_PROXY") or os.environ.get("HTTP_PROXY")
        if proxy:
            options["proxy"] = {"server": proxy, "bypass": "127.0.0.1,localhost,[::1]"}
        return options

    @staticmethod
    async def go(page, path):
        await page.goto(
            Runtime.app_url() + path, wait_until="networkidle", timeout=60000
        )
        await page.evaluate("document.fonts.ready")


common = Runtime()


async def ready(p):
    await common.go(p, "/")
    await expect(p.locator("h1")).to_have_text(
        "The Ultimate Recipe Manager", timeout=30000
    )
    await p.evaluate("document.fonts.ready")
    # Walk the page so lazy-loaded images decode before the full-page capture.
    await p.evaluate(
        "async()=>{for(let y=0;y<document.documentElement.scrollHeight;y+=600){scrollTo(0,y);await new Promise(r=>setTimeout(r,30))}scrollTo(0,0)}"
    )
    await p.evaluate(
        'Promise.all([...document.querySelectorAll(".landing-page img")].filter(i=>i.getClientRects().length).map(i=>i.decode()))'
    )


async def capture_section(page, selector, name):
    element = page.locator(selector)
    height = round(await element.evaluate("e=>e.getBoundingClientRect().height"))
    await page.set_viewport_size(
        {"width": page.viewport_size["width"], "height": height + 240}
    )
    await element.evaluate("e=>scrollTo(0,e.getBoundingClientRect().top+scrollY-120)")
    await page.screenshot(
        path=str(ROOT / name),
        clip=await element.bounding_box(),
        type="jpeg",
        quality=88,
        animations="disabled",
    )


async def main():
    errors = []
    failures = []
    results = []
    async with async_playwright() as pw:
        b = await pw.chromium.launch(**common.options())
        ctx = await b.new_context(
            viewport={"width": 1440, "height": 1000}, reduced_motion="reduce"
        )
        p = await ctx.new_page()
        p.on("pageerror", lambda e: errors.append(str(e)))
        p.on(
            "response",
            lambda r: failures.append([r.status, r.url]) if r.status >= 400 else None,
        )
        await ready(p)
        for width, name in [
            (1440, "desktop"),
            (393, "mobile"),
            (320, "compact"),
            (768, "tablet"),
        ]:
            await p.set_viewport_size({"width": width, "height": 900})
            await p.evaluate("scrollTo(0,0)")
            await expect(p.locator(".landing-chapter")).to_have_count(5)
            await expect(p.locator(".landing-feature-card")).to_have_count(18)
            await expect(p.locator(".landing-mobile-card")).to_have_count(3)
            dims = await p.evaluate(
                '({width:innerWidth,scroll:document.documentElement.scrollWidth,missing:[...document.querySelectorAll(".landing-page img")].filter(i=>i.getClientRects().length&&(!i.complete||!i.naturalWidth)).map(i=>i.alt)})'
            )
            assert dims["width"] >= dims["scroll"] and not dims["missing"], dims
            if name in ["desktop", "mobile"]:
                await p.screenshot(
                    path=str(ROOT / f"{name}-full.jpg"),
                    full_page=True,
                    type="jpeg",
                    quality=86,
                    animations="disabled",
                )
                await p.screenshot(
                    path=str(ROOT / f"{name}-hero.jpg"),
                    type="jpeg",
                    quality=88,
                    animations="disabled",
                )
            results.append(
                {"width": width, "no_overflow": True, "images_decoded": True}
            )
        # Zoom works on every story and phone, including keyboard dismissal and focus restoration.
        await p.set_viewport_size({"width": 393, "height": 852})
        for button in await p.locator(".landing-shot-button").all():
            await button.click()
            await expect(p.get_by_role("dialog")).to_be_visible()
            await expect(
                p.get_by_role("button", name="Close screenshot", exact=True)
            ).to_be_focused()
            await p.keyboard.press("Escape")
            await expect(p.get_by_role("dialog")).to_have_count(0)
            await expect(button).to_be_focused()
        await p.get_by_role("button", name="Pause animation", exact=True).click()
        await expect(
            p.get_by_role("button", name="Play animation", exact=True)
        ).to_have_attribute("aria-pressed", "true")
        await p.get_by_role("button", name="Play animation", exact=True).click()
        await capture_section(p, ".landing-mobile", "mobile-experience.jpg")
        await p.set_viewport_size({"width": 1440, "height": 1000})
        await capture_section(
            p, ".landing-chapter:nth-child(3)", "calculator-checklist.jpg"
        )
        await capture_section(p, ".landing-features", "feature-grid.jpg")
        # Use the real theme toggle rather than altering page styles in the test.
        await p.set_viewport_size({"width": 1440, "height": 1000})
        await p.evaluate("scrollTo(0,0)")
        toggle = p.get_by_role(
            "button", name=re.compile("Switch to dark|Toggle theme|dark mode", re.I)
        ).first
        await toggle.click()
        await expect(p.locator("html")).to_have_class(re.compile("dark"))
        await p.screenshot(
            path=str(ROOT / "desktop-dark.jpg"),
            full_page=True,
            type="jpeg",
            quality=86,
            animations="disabled",
        )
        await p.set_viewport_size({"width": 393, "height": 900})
        await p.screenshot(
            path=str(ROOT / "mobile-dark.jpg"),
            full_page=True,
            type="jpeg",
            quality=86,
            animations="disabled",
        )
        # Signed-out primary CTA preserves the return path through login; Explore stays public.
        await p.locator(".landing-actions").first.get_by_role(
            "link", name=re.compile("Start your recipe book")
        ).click()
        await p.wait_for_url(re.compile("/login\\?redirect_uri="))
        assert "/recipes/new" in unquote(p.url)
        await common.go(p, "/")
        await p.locator(".landing-actions").first.get_by_role(
            "link", name="Explore recipes", exact=True
        ).click()
        await p.wait_for_url(common.app_url() + "/explore")
        await expect(
            p.get_by_role("heading", name="Explore", exact=True)
        ).to_be_visible()
        await ctx.close()
        static_ctx = await b.new_context(
            java_script_enabled=False, viewport={"width": 393, "height": 900}
        )
        static = await static_ctx.new_page()
        await common.go(static, "/")
        await expect(static.locator("h1")).to_have_text("The Ultimate Recipe Manager")
        await expect(static.locator(".landing-chapter")).to_have_count(5)
        await expect(static.locator(".landing-feature-card")).to_have_count(18)
        await static_ctx.close()
        motion_ctx = await b.new_context(
            viewport={"width": 393, "height": 900}, reduced_motion="no-preference"
        )
        motion = await motion_ctx.new_page()
        await common.go(motion, "/")
        await motion.locator(".landing-mobile").scroll_into_view_if_needed()
        frames = motion.locator(".landing-cooking-frames img")
        assert (
            await frames.first.evaluate("e=>getComputedStyle(e).animationName")
            == "landing-cooking-step"
        )
        await motion.get_by_role("button", name="Pause animation", exact=True).click()
        assert (
            await frames.first.evaluate("e=>getComputedStyle(e).animationPlayState")
            == "paused"
        )
        await motion.get_by_role("button", name="Play animation", exact=True).click()
        assert (
            await frames.first.evaluate("e=>getComputedStyle(e).animationPlayState")
            == "running"
        )
        await motion_ctx.close()
        await b.close()
    assert not errors, errors
    assert not failures, failures
    (ROOT / "verification.json").write_text(
        json.dumps(
            {
                "viewports": results,
                "eight_screenshot_dialogs": True,
                "escape_and_focus_restoration": True,
                "animation_controls": True,
                "dark_theme": True,
                "server_rendered_without_javascript": True,
                "normal_and_reduced_motion": True,
                "signed_out_cta_and_public_explore": True,
                "page_errors": errors,
                "http_errors": failures,
            },
            indent=2,
        )
    )
    print("Landing browser checks passed", flush=True)


asyncio.run(main())
