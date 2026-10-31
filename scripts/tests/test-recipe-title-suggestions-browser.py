#!/usr/bin/env python3
"""Live recipe-title API and browser checks against the local mock-auth stack.

Creates synthetic local fixtures and removes those fixtures on completion.
"""
import argparse
import asyncio
import json
import os
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


async def main(args):
    from playwright.async_api import async_playwright, expect

    expect.set_options(timeout=20000)
    base = args.base_url.rstrip('/')
    assert urlsplit(base).hostname in {'127.0.0.1', 'localhost', '::1'}
    assert re.fullmatch(r'noms-[a-z0-9][a-z0-9-]*', args.instance)
    label = subprocess.check_output(['docker', 'inspect', args.instance + '-postgres-1', '--format',
                                    '{{index .Config.Labels "com.noms.dev-environment"}}'], text=True).strip()
    assert label == 'true', 'Fixtures require the named local development database'
    args.output.mkdir(parents=True, exist_ok=True)
    suffix = uuid.uuid4().hex[:10]
    needle = 'titleprobe' + suffix
    checks, errors, requests, fixture_ids = [], [], [], []
    other = str(uuid.uuid4())

    def sql(statement, **values):
        command = ['docker', 'exec', '-i', args.instance + '-postgres-1', 'psql', '-U', 'noms', '-d', 'noms', '-Atq', '-v', 'ON_ERROR_STOP=1']
        for key, value in values.items():
            command += ['-v', key + '=' + value]
        return subprocess.run(command, input=statement, text=True, capture_output=True, check=True).stdout.strip()

    def passed(message):
        checks.append(message)
        print('PASS:', message, flush=True)

    def search_text(url):
        return parse_qs(urlsplit(url).query).get('q', [''])[0]

    async with async_playwright() as playwright:
        options = dict(executable_path=args.chromium, headless=True, args=['--disable-dev-shm-usage'])
        proxy = os.environ.get('HTTPS_PROXY') or os.environ.get('HTTP_PROXY')
        if proxy:
            options['proxy'] = dict(server=proxy, bypass='127.0.0.1,localhost,[::1]')
        browser = await playwright.chromium.launch(**options)
        context = await browser.new_context(viewport={'width': 1440, 'height': 1000}, has_touch=True)
        public = await browser.new_context(viewport={'width': 1440, 'height': 1000})
        for client in [context, public]:
            await client.route(re.compile(r'https://fonts\.(googleapis|gstatic)\.com/'), lambda route: route.abort())
        page = await context.new_page()
        page.on('pageerror', lambda error: errors.append(str(error)))
        page.on('request', lambda request: requests.append(request.post_data_json)
                if '/api/suggest_recipe_titles' in request.url else None)
        try:
            await page.goto(base + '/login', wait_until='networkidle')
            await page.get_by_role('link', name='Continue with Google').click()
            await page.locator('input[name=username]').fill('titles-' + suffix)
            await page.locator('textarea[name=claims]').fill(json.dumps({
                'email': f'titles-{suffix}@example.test', 'email_verified': True, 'name': 'Title Suggestions Test',
            }))
            await page.locator('input[type=submit]').click()
            await page.wait_for_url(base + '/**')
            profile = await (await context.request.get(base + '/api/user_profile')).json()
            owner = str(uuid.UUID(profile['current_user_id']))
            sql("INSERT INTO users(id,username,display_name,email) VALUES(:'id'::uuid,:'name','Other title fixture',:'email');",
                id=other, name='title_other_' + suffix, email=f'other-{suffix}@example.test')
            records = []

            def add(title, visibility='public', user=other, description=''):
                ident = str(uuid.uuid4())
                fixture_ids.append(ident)
                records.append(dict(id=ident, user_id=user, title=title, visibility=visibility, description=description))
                return ident

            add(needle)
            chicken = needle + ' Chicken curry'
            for _ in range(12):
                add(chicken)
            add('  ' + needle.upper() + '   CHICKEN   CURRY  ')
            add(chicken + ' with spinach')
            for index in range(10):
                add(needle + f' Dish {index}')
            own = needle + ' My private curry'
            hidden = needle + ' Hidden curry'
            unlisted = needle + ' Unlisted curry'
            add(own, 'private', owner)
            add(hidden, 'private')
            add(unlisted, 'unlisted')
            add('Generic dessert ' + suffix, description=needle + ' bodyonly')
            special = needle + ' R&D + pie #1'
            add(special)
            sql("""INSERT INTO recipes(id,user_id,title,visibility,description,ingredients,instructions)
                SELECT id,user_id,title,visibility,description,
                    jsonb_build_array(jsonb_build_object('name',description)),
                    jsonb_build_array(jsonb_build_object('text',description))
                FROM jsonb_to_recordset(:'records'::jsonb) AS r(id uuid,user_id uuid,title text,visibility text,description text);""",
                records=json.dumps(records))

            await page.goto(base + '/dashboard', wait_until='networkidle')
            header = page.get_by_role('search', name='Site search', exact=True).get_by_role('combobox', name='Search Noms', exact=True)
            async with page.expect_request(lambda r: r.method == 'POST' and '/api/suggest_recipe_titles' in r.url) as pending:
                await header.fill(needle)
            endpoint = (await pending.value).url

            async def api(text, public_only=False, client=context, expected_ok=True):
                response = await client.request.post(endpoint, data=json.dumps(dict(text=text, public_only=public_only)),
                                                     headers={'Content-Type': 'application/json', 'Origin': base})
                assert response.ok == expected_ok, (response.status, await response.text())
                if not expected_ok:
                    return
                rows = await response.json()
                assert isinstance(rows, list) and len(rows) <= 8 and all(isinstance(row, str) for row in rows), rows
                assert len({row.lower() for row in rows}) == len(rows), rows
                return rows

            rows = await api(needle)
            assert len(rows) == 8 and rows[0] == needle
            curry = await api(needle + ' chicken curry')
            assert len(curry) == 2 and curry[0].lower() == chicken.lower(), curry
            chosen = curry[0]
            assert await api(own) == [own]
            assert await api(own, True) == []
            assert await api(own, client=public) == []
            assert await api(hidden) == [] and await api(unlisted) == []
            assert await api(needle + ' bodyonly') == []
            assert await api(needle, client=public)
            await api('a', expected_ok=False)
            await api('x' * 257, expected_ok=False)
            passed('API: strings only, eight distinct titles, title-only matching, exact ranking, validation and public/owner visibility')

            live = str(uuid.uuid4())
            fixture_ids.append(live)
            old_title, new_title = needle + ' Live fresh', needle + ' Live renamed'
            sql("INSERT INTO recipes(id,user_id,title,visibility) VALUES(:'id'::uuid,:'owner'::uuid,:'title','public');",
                id=live, owner=owner, title=old_title)
            assert await api(old_title) == [old_title]
            sql("UPDATE recipes SET title=:'title' WHERE id=:'id'::uuid;", id=live, title=new_title)
            assert await api(old_title) == [] and await api(new_title) == [new_title]
            sql("UPDATE recipes SET visibility='private' WHERE id=:'id'::uuid;", id=live)
            assert await api(new_title, client=public) == [] and await api(new_title) == [new_title]
            sql("DELETE FROM recipes WHERE id=:'id'::uuid;", id=live)
            assert await api(new_title) == []
            passed('API: new recipes, renames, visibility changes and deletions take effect without refresh jobs')

            async def choose(field, text, expected, method='keyboard'):
                await field.fill(text)
                controls = await field.get_attribute('aria-controls')
                menu = page.locator(f'[id="{controls}"]')
                option = menu.get_by_role('option', name=expected, exact=True)
                await expect(option).to_be_visible()
                assert await option.inner_text() == expected
                if method == 'keyboard':
                    labels = await menu.get_by_role('option').all_text_contents()
                    for _ in range(labels.index(expected) + 1):
                        await field.press('ArrowDown')
                    assert await field.get_attribute('aria-activedescendant') == await option.get_attribute('id')
                    await field.press('Enter')
                elif method == 'touch':
                    await option.tap()
                else:
                    await option.click()
                await page.wait_for_url(lambda url: search_text(str(url)) == expected)
                assert urlsplit(page.url).path == '/search', page.url

            await choose(header, needle + ' chick', chosen)
            await expect(page.locator('#search-query')).to_have_value(chosen)
            await expect(page.locator('.search-recipe-grid .recipe-card').first).to_be_visible()
            assert await page.locator('.search-recipe-grid .recipe-card').count() >= 2
            passed('navigation: keyboard choice submits the title to search and shows multiple matching recipes')

            main_field = page.locator('#search-query')
            before = page.url
            await main_field.fill(needle + ' chick')
            await expect(page.get_by_role('option', name=chosen, exact=True)).to_be_visible()
            assert page.url == before, 'Typing a query must not navigate before submission'
            await main_field.press('Escape')
            await expect(main_field).to_have_value(needle + ' chick')
            await expect(main_field).to_have_attribute('aria-expanded', 'false')
            await choose(main_field, needle + ' R&D', special, 'mouse')
            assert search_text(page.url) == special
            passed('main search: text-only mouse choice, encoded punctuation, dismissal and submit-only navigation')

            await main_field.fill(needle + ' chick')
            await page.get_by_role('navigation', name='Search type').get_by_role('button', name='Recipes', exact=True).click()
            await page.wait_for_url(lambda url: search_text(str(url)) == needle + ' chick' and parse_qs(urlsplit(str(url)).query).get('scope') == ['recipes'])
            await choose(main_field, needle + ' chick', chosen, 'mouse')
            assert parse_qs(urlsplit(page.url).query).get('scope') == ['recipes']
            passed('search scopes: changing scope preserves draft text and selecting a title preserves the selected scope')

            before_requests = len(requests)
            for text in [needle[:4], needle[:7], needle[:11], needle + ' chick']:
                await main_field.fill(text)
            await expect(page.get_by_role('option', name=chosen, exact=True)).to_be_visible()
            await page.wait_for_timeout(250)
            recent = requests[before_requests:]
            assert len(recent) == 1 and recent[0]['text'] == needle + ' chick', recent
            passed('client: rapid typing sends only the final debounced title request')
            before_navigation = len(requests)
            await main_field.press('ArrowDown')
            await page.wait_for_timeout(250)
            await main_field.press('ArrowUp')
            await page.wait_for_timeout(250)
            assert len(requests) == before_navigation, 'Highlighting options must not refetch titles'
            passed('client: keyboard highlighting does not issue extra title lookups')

            # Delay an old request until after a newer response has arrived.
            async def delayed(route):
                payload = route.request.post_data_json
                if payload['text'].endswith(' stale'):
                    await asyncio.sleep(0.8)
                    try:
                        await route.fulfill(status=200, content_type='application/json', body=json.dumps([needle + ' Stale result']))
                    except Exception:
                        pass  # The obsolete request may already have been aborted.
                elif payload['text'].endswith(' fresh'):
                    await route.fulfill(status=200, content_type='application/json', body=json.dumps([needle + ' Fresh result']))
                elif payload['text'].endswith(' failed'):
                    await route.fulfill(status=503, content_type='text/plain', body='Unavailable')
                else:
                    await route.continue_()

            await context.route('**/api/suggest_recipe_titles*', delayed)
            async with page.expect_request(lambda r: '/api/suggest_recipe_titles' in r.url and r.post_data_json['text'].endswith(' stale')):
                await main_field.fill(needle + ' stale')
            await main_field.fill(needle + ' fresh')
            await expect(page.get_by_role('option', name=needle + ' Fresh result', exact=True)).to_be_visible()
            await page.wait_for_timeout(1000)
            await expect(page.get_by_role('option', name=needle + ' Stale result', exact=True)).to_have_count(0)
            await main_field.fill(needle + ' failed')
            await expect(page.get_by_text('Suggestions unavailable. Press Enter to search.', exact=True)).to_be_visible()
            await main_field.press('Enter')
            await page.wait_for_url(lambda url: search_text(str(url)) == needle + ' failed')
            await context.unroute('**/api/suggest_recipe_titles*', delayed)
            passed('client: stale responses are discarded and failed suggestions still allow ordinary search submission')

            await page.set_viewport_size({'width': 390, 'height': 844})
            await page.get_by_role('button', name='Open navigation menu', exact=True).click()
            drawer = page.get_by_role('dialog')
            mobile = drawer.get_by_role('combobox', name='Search Noms', exact=True)
            await choose(mobile, needle + ' chick', chosen, 'touch')
            await expect(drawer).to_have_count(0)
            await expect(page.locator('#search-query')).to_have_value(chosen)
            await page.screenshot(path=str(args.output / 'mobile-search.png'), full_page=True)
            passed('mobile drawer: touch selection submits text and closes the drawer without opening a recipe')

            public_page = await public.new_page()
            public_page.on('pageerror', lambda error: errors.append(str(error)))
            await public_page.goto(base + '/', wait_until='networkidle')
            landing = public_page.get_by_role('combobox', name='Search public recipes', exact=True)
            await landing.fill(needle + ' chick')
            await public_page.get_by_role('option', name=chosen, exact=True).click()
            await public_page.wait_for_url(lambda url: urlsplit(str(url)).path == '/explore' and search_text(str(url)) == chosen)
            discovery = public_page.get_by_role('combobox', name='Search public recipes', exact=True)
            await expect(discovery).to_have_value(chosen)
            await discovery.fill(needle + ' chick')
            await expect(public_page.get_by_role('option', name=chosen, exact=True)).to_be_visible()
            await public_page.screenshot(path=str(args.output / 'public-titles.png'), full_page=True)
            await public_page.get_by_role('option', name=chosen, exact=True).click()
            await expect(discovery).to_have_value(chosen)
            passed('anonymous landing and Explore: public-title completion submits to public search with no recipe IDs or authors')
            assert not errors, errors
            (args.output / 'result.json').write_text(json.dumps(dict(checks=checks, page_errors=errors, suggestion_requests=len(requests)), indent=2))
        except Exception:
            print('PAGE ERRORS:', errors, flush=True)
            await page.screenshot(path=str(args.output / 'failure.png'), full_page=True)
            raise
        finally:
            if fixture_ids:
                sql("DELETE FROM recipes WHERE id IN (SELECT value::uuid FROM jsonb_array_elements_text(:'ids'::jsonb));", ids=json.dumps(fixture_ids))
            sql("DELETE FROM users WHERE id=:'id'::uuid;", id=other)
            await browser.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--instance', required=True)
    parser.add_argument('--base-url', required=True)
    parser.add_argument('--chromium', default='/usr/bin/chromium')
    parser.add_argument('--output', type=Path, default=Path('/tmp/noms-recipe-title-suggestions-browser'))
    asyncio.run(main(parser.parse_args()))
