#!/usr/bin/env python3
"""Check an already-loaded ingredient seed through the local mock-auth UI.

Requires Chromium, Python Playwright, and a running disposable Codex cloud stack.
Creates one private autosaved draft; never publishes a recipe or loads a seed.
"""
import argparse
import asyncio
import json
import os
from pathlib import Path
import re
import shutil
import time
from urllib.parse import urlsplit
import uuid


async def check(args):
    from playwright.async_api import async_playwright, expect

    metadata = Path(os.environ.get('NOMS_RUNTIME_ROOT', '/tmp/noms-dev')) / args.instance / 'metadata'
    values = dict(line.split('=', 1) for line in metadata.read_text().splitlines() if '=' in line)
    base = values['APP_URL']
    if values.get('STATE') != 'ready' or urlsplit(base).hostname != '127.0.0.1':
        raise RuntimeError('Expected a ready loopback development instance')
    entry = next(e for e in json.loads(args.seed.read_text())['entries'] if e['id'] == str(args.entry_id))
    if entry['status'] != 'active' or entry['locale'] != 'en':
        raise RuntimeError('Expected an active English ingredient')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True, mode=0o700)
    errors, wasm, checks = [], [], []
    title = 'FoodOn UI smoke ' + uuid.uuid4().hex[:12]
    custom = 'My custom cumin blend'

    async with async_playwright() as playwright:
        options = dict(executable_path=args.chromium or shutil.which('chromium'),
                       headless=True, args=['--disable-dev-shm-usage'])
        proxy = os.environ.get('HTTPS_PROXY') or os.environ.get('HTTP_PROXY')
        if proxy:
            options['proxy'] = dict(server=proxy, bypass='127.0.0.1,localhost,[::1]')
        browser = await playwright.chromium.launch(**options)
        try:
            context = await browser.new_context(viewport=dict(width=1440, height=1000))
            page = await context.new_page()
            page.on('pageerror', lambda error: errors.append(str(error)))
            page.on('response', lambda response: wasm.append(response.status)
                    if urlsplit(response.url).path.endswith('.wasm') else None)
            await page.goto(base, wait_until='networkidle')
            await page.get_by_role('button', name='Toggle theme', exact=True).click()
            await page.wait_for_function("document.documentElement.classList.contains('dark')")
            assert await page.evaluate("localStorage.getItem('theme')") == 'dark'
            checks.append('WASM hydration and theme persistence')
            await page.goto(base + '/login?redirect_uri=%2Frecipes%2Fnew', wait_until='networkidle')
            await page.get_by_role('link', name='Continue with GitHub', exact=True).click()
            await page.locator('input[name="username"]').wait_for()
            # Refuse to submit synthetic credentials to any real OAuth provider.
            origin = urlsplit(page.url)
            if origin.hostname != '127.0.0.1' or str(origin.port) != values['MOCK_OAUTH_PORT']:
                raise RuntimeError('Expected this instance\'s local mock OAuth server')
            await page.locator('input[name="username"]').fill('foodon-ui-smoke')
            await page.locator('input[type="submit"]').click()
            await page.wait_for_url(base + '/recipes/new')
            await page.wait_for_load_state('networkidle')
            # The OAuth callback starts a new document; wait for its hydration too.
            await page.get_by_role('button', name='Toggle theme', exact=True).click()
            await page.wait_for_function("!document.documentElement.classList.contains('dark')")
            await page.get_by_role('textbox', name='Recipe name', exact=True).fill(title)
            await page.get_by_role('button', name='+ Add Ingredient', exact=True).click()
            field = page.get_by_role('combobox', name='Ingredient 1 name', exact=True)

            def is_saved(response, name, identifier):
                if response.status != 200 or not re.search(r'/api/(create|save)_recipe_draft', response.url):
                    return False
                data = response.request.post_data_json
                rows = data.get('editor', {}).get('ingredients', [])
                return (len(rows) == 1 and rows[0]['name'] == name
                        and rows[0].get('vocabulary_id') == identifier)

            async def saved(response_info, name, identifier):
                response = await response_info.value
                data = await response.json()
                rows = data['editor']['ingredients']
                assert rows[0]['name'] == name and rows[0].get('vocabulary_id') == identifier
                return data

            cache_name = 'noms-ingredient-vocabulary-v1'
            cache_key = '/__noms_cache/ingredient-vocabulary-v1'
            await page.wait_for_function("""async ([name, key]) =>
                Boolean(await (await caches.open(name)).match(key))""", arg=[cache_name, cache_key])
            lookups, catalogs = [], []
            page.on('request', lambda request: lookups.append(request.url)
                    if '/api/ingredient_suggestions' in request.url else None)
            page.on('request', lambda request: catalogs.append(request.url)
                    if '/api/vocabulary/ingredients/' in request.url and '/version.json' not in request.url else None)
            await field.fill(args.query)
            option = page.get_by_role('option', name=entry['label'], exact=True)
            await expect(option).to_be_visible(timeout=15000)
            # Keep source IDs, technical metadata and definitions out of options.
            assert await option.inner_text() == entry['label']
            await field.scroll_into_view_if_needed()
            await page.screenshot(path=str(output / 'ingredient-suggestion.png'), full_page=True)
            async with page.expect_response(lambda r: is_saved(r, entry['label'], str(args.entry_id)), timeout=20000) as response_info:
                await field.press('ArrowDown')
                await expect(option).to_have_attribute('aria-selected', 'true')
                await field.press('Enter')
                await expect(field).to_have_value(entry['label'])
                await expect(page.get_by_role('listbox')).to_have_count(0)
            selected = await saved(response_info, entry['label'], str(args.entry_id))
            assert not lookups, 'Loaded vocabulary should search locally'
            draft_url = base + '/recipes/drafts/' + selected['id'] + '/edit'
            checks.append('Alias query, label-only option, keyboard selection, canonical UUID saved')

            await page.goto(draft_url, wait_until='networkidle')
            field = page.get_by_role('combobox', name='Ingredient 1 name', exact=True)
            await expect(field).to_have_value(entry['label'])
            await field.focus()
            await expect(page.get_by_role('option', name=entry['label'], exact=True)).to_be_visible()
            assert not catalogs, 'Editor reload should reuse the stored catalog'
            assert not lookups, 'Persisted vocabulary should search locally'
            checks.append('Imported selection survives page reload')
            await page.screenshot(path=str(output / 'ingredient-selected.png'), full_page=True)

            async with page.expect_response(lambda r: is_saved(r, custom, None), timeout=20000) as response_info:
                await field.fill(custom)
            await saved(response_info, custom, None)
            checks.append('Editing selection clears canonical UUID and saves free text')

            # Without a catalog the existing authenticated server fallback works.
            await page.evaluate('(name) => caches.delete(name)', cache_name)
            await page.route('**/api/vocabulary/ingredients/**', lambda route: route.abort())
            await page.reload(wait_until='networkidle')
            field = page.get_by_role('combobox', name='Ingredient 1 name', exact=True)
            async with page.expect_response(lambda r: '/api/ingredient_suggestions' in r.url
                                            and r.request.post_data_json.get('text') == args.query) as lookup_info:
                await field.fill(args.query)
            lookup_url = (await lookup_info.value).url
            await expect(page.get_by_role('option', name=entry['label'], exact=True)).to_be_visible()
            checks.append('Missing catalog uses the authenticated server fallback')

            # Confirm lookup failure is optional and does not block draft saving.
            await page.route('**/api/ingredient_suggestions*', lambda route: route.abort())
            fallback = 'My unavailable-lookup ingredient'
            async with page.expect_response(lambda r: is_saved(r, fallback, None), timeout=20000) as response_info:
                await field.fill(fallback)
                await expect(page.get_by_role('status').filter(has_text='Suggestions unavailable')).to_be_visible()
            await saved(response_info, fallback, None)
            await page.unroute('**/api/ingredient_suggestions*')
            checks.append('Lookup failure still permits free text autosave')
            await page.unroute('**/api/vocabulary/ingredients/**')
            await page.reload(wait_until='networkidle')
            field = page.get_by_role('combobox', name='Ingredient 1 name', exact=True)

            # Leave the probe visibly selected for manual inspection of this draft.
            await field.fill(args.query)
            option = page.get_by_role('option', name=entry['label'], exact=True)
            await expect(option).to_be_visible(timeout=15000)
            async with page.expect_response(lambda r: is_saved(r, entry['label'], str(args.entry_id)), timeout=20000) as response_info:
                await option.click()
            await saved(response_info, entry['label'], str(args.entry_id))
            checks.append('Mouse selection also saves the canonical UUID')
            quality = []
            if args.quality_checks:
                expected_cases = json.loads(args.quality_checks.read_text())
                entries = {e['id']: e for e in json.loads(args.seed.read_text())['entries']}
                for case in expected_cases:
                    start = time.perf_counter()
                    response = await context.request.post(
                        lookup_url, data=json.dumps(dict(text=case['query'])),
                        headers={'Content-Type': 'application/json', 'Origin': base, 'Referer': draft_url})
                    elapsed = round((time.perf_counter() - start) * 1000, 3)
                    assert response.status == 200, 'Lookup failed for ' + case['query']
                    rows = await response.json()
                    assert len(rows) <= 8 and len({r['id'] for r in rows}) == len(rows)
                    for row in rows:
                        assert entries[row['id']]['status'] == 'active'
                        assert entries[row['id']]['label'] == row['label']
                    if case.get('empty'):
                        assert not rows, 'Expected no matches for ' + case['query']
                    if case.get('first'):
                        assert rows and rows[0]['label'] == case['first'], (case, rows)
                    for forbidden in case.get('forbidden', []):
                        assert all(row['label'] != forbidden for row in rows)
                    quality.append(dict(query=case['query'], labels=[r['label'] for r in rows], milliseconds=elapsed))
                checks.append(f'{len(quality)} authenticated lookup quality/limit/escaping checks')
            if not wasm or any(status != 200 for status in wasm) or errors:
                raise RuntimeError('WASM delivery or browser errors: ' + json.dumps(errors))
            result = dict(passed=True, checks=checks, entry_id=str(args.entry_id), label=entry['label'],
                          draft_id=selected['id'], draft_title=title, draft_url=draft_url,
                          browser=browser.version, wasm_statuses=wasm, page_errors=errors,
                          quality_checks=quality,
                          screenshots=['ingredient-suggestion.png', 'ingredient-selected.png'])
            (output / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
            print(json.dumps(result, indent=2))
        except Exception:
            if 'page' in locals():
                await page.screenshot(path=str(output / 'failure.png'), full_page=True)
            raise
        finally:
            await browser.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--instance', default='noms-noms017')
    parser.add_argument('--seed', type=Path, required=True, help='The complete seed already loaded into this instance')
    parser.add_argument('--entry-id', type=uuid.UUID, required=True)
    parser.add_argument('--query', required=True, help='Alias or prefix expected to match the selected entry')
    parser.add_argument('--quality-checks', type=Path, help='Optional JSON query/expected-first/empty cases')
    parser.add_argument('--chromium')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'noms-[a-z0-9][a-z0-9-]*', args.instance):
        parser.error('Expected a normalized noms- instance name')
    asyncio.run(check(args))


if __name__ == '__main__':
    main()
