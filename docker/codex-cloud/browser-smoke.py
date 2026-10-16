#!/usr/bin/env python3
"""Headless browser smoke check and screenshots of a local Noms instance."""
import argparse
import asyncio
import json
import os
from pathlib import Path
import shutil
from urllib.parse import urlsplit


async def check(args):
    from playwright.async_api import async_playwright
    metadata = Path(os.environ.get('NOMS_RUNTIME_ROOT', '/tmp/noms-dev')) / args.instance / 'metadata'
    values = dict(line.split('=', 1) for line in metadata.read_text().splitlines() if '=' in line)
    base = values['APP_URL']
    if values.get('STATE') != 'ready' or urlsplit(base).hostname not in ('127.0.0.1', 'localhost', '::1'):
        raise RuntimeError('Expected a ready local instance; refusing to browse an unexpected origin')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True, mode=0o700)
    errors, wasm = [], []
    async with async_playwright() as playwright:
        executable = args.chromium or shutil.which('chromium') or shutil.which('chromium-browser')
        if not executable:
            raise RuntimeError('Chromium is not installed; see README browser prerequisites')
        options = dict(executable_path=executable, headless=True, args=['--disable-dev-shm-usage'])
        proxy = os.environ.get('HTTPS_PROXY') or os.environ.get('HTTP_PROXY')
        if proxy:
            options['proxy'] = dict(server=proxy, bypass='127.0.0.1,localhost,[::1]')
        browser = await playwright.chromium.launch(**options)
        try:
            context = await browser.new_context(viewport=dict(width=1440, height=1000))
            page = await context.new_page()
            page.on('pageerror', lambda error: errors.append(str(error)))
            page.on('response', lambda response: wasm.append(response.status) if urlsplit(response.url).path.endswith('.wasm') else None)
            response = await page.goto(base, wait_until='networkidle', timeout=60000)
            if response.status != 200:
                raise RuntimeError('Home page HTTP ' + str(response.status))
            await page.get_by_role('heading', name='A cozy home for your recipes.').wait_for()
            await page.screenshot(path=str(output / 'home.png'), full_page=True)
            await page.get_by_role('button', name='Toggle theme', exact=True).click()
            await page.wait_for_function("document.documentElement.classList.contains('dark')", timeout=15000)
            if await page.evaluate("localStorage.getItem('theme')") != 'dark':
                raise RuntimeError('Theme did not persist after client-side interaction')
            await page.screenshot(path=str(output / 'home-dark.png'), full_page=True)
            await page.locator('a[href="/login"]').first.click()
            await page.wait_for_url('**/login', timeout=15000)
            await page.get_by_role('heading', name='Welcome back').wait_for()
            await page.screenshot(path=str(output / 'login.png'), full_page=True)
            if not wasm or any(status != 200 for status in wasm) or errors:
                raise RuntimeError('WASM failed or browser page errors occurred: ' + json.dumps(errors))
            result = dict(passed=True, browser=browser.version, home_status=response.status,
                          wasm_statuses=wasm, hydration='theme DOM/localStorage changed', navigation='login',
                          screenshots=['home.png', 'home-dark.png', 'login.png'], page_errors=errors)
            (output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
            print(json.dumps(result, indent=2))
        finally:
            await browser.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--instance', default='noms-cloud')
    parser.add_argument('--chromium', default=os.environ.get('NOMS_CHROMIUM'))
    parser.add_argument('--output', type=Path, default=Path(os.environ.get('NOMS_BROWSER_OUTPUT', '/tmp/noms-browser-artifacts')))
    args = parser.parse_args()
    if not args.instance.startswith('noms-') or '/' in args.instance or args.instance in ('.', '..'):
        parser.error('Use a normalized noms- instance name, without path separators')
    asyncio.run(check(args))


if __name__ == '__main__':
    main()
