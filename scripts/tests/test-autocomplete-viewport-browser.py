#!/usr/bin/env python3
"""Exercise shared autocomplete positioning with simulated mobile keyboards.

Chromium does not open an OS keyboard headlessly. Override only VisualViewport
measurements and dispatch its real resize/scroll events to exercise the app's
production placement, focus, scrolling and selection code.
"""
import argparse
import asyncio
import json
import os
import re
import uuid
from pathlib import Path
from urllib.parse import urlsplit


VIEWPORT = """(() => {
    const viewport = window.visualViewport;
    const values = {};
    for (const key of ['height', 'offsetTop', 'offsetLeft']) {
        const original = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(viewport), key).get;
        Object.defineProperty(viewport, key, {get: () => values[key] ?? original.call(viewport)});
    }
    window.keyboardViewport = (height, offsetTop = 0, offsetLeft = 0) => {
        Object.assign(values, {height, offsetTop, offsetLeft});
        viewport.dispatchEvent(new Event('resize'));
        viewport.dispatchEvent(new Event('scroll'));
    };
})();"""


async def main(args):
    from playwright.async_api import async_playwright, expect

    expect.set_options(timeout=20000)
    base = args.base_url.rstrip('/')
    assert urlsplit(base).hostname in {'127.0.0.1', 'localhost', '::1'}
    args.output.mkdir(parents=True, exist_ok=True)
    suffix = uuid.uuid4().hex[:10]
    checks, errors, measurements = [], [], []
    async with async_playwright() as playwright:
        options = dict(executable_path=args.chromium, headless=True, args=['--disable-dev-shm-usage'])
        proxy = os.environ.get('HTTPS_PROXY') or os.environ.get('HTTP_PROXY')
        if proxy:
            options['proxy'] = dict(server=proxy, bypass='127.0.0.1,localhost,[::1]')
        browser = await playwright.chromium.launch(**options)
        context = await browser.new_context(viewport={'width': 390, 'height': 844}, has_touch=True)
        await context.add_init_script(VIEWPORT)
        await context.route(re.compile(r'https://fonts\.(googleapis|gstatic)\.com/'), lambda route: route.abort())
        page = await context.new_page()
        page.on('pageerror', lambda error: errors.append(str(error)))

        def passed(message):
            checks.append(message)
            print('PASS:', message, flush=True)

        async def geometry(field, placement):
            ident = await field.get_attribute('aria-controls')
            await page.wait_for_function("([id, at]) => document.getElementById(id)?.dataset.placement === at", arg=[ident, placement])
            menu = page.locator(f'[id="{ident}"]')
            await expect(menu).to_be_visible()
            data = await menu.evaluate("""el => {
                const r = el.getBoundingClientRect(), v = visualViewport;
                const options = [...el.querySelectorAll('[role=option]')];
                return {top:r.top, bottom:r.bottom, left:r.left, right:r.right,
                    height:r.height, scrollHeight:el.scrollHeight, clientHeight:el.clientHeight,
                    viewTop:v.offsetTop, viewBottom:v.offsetTop+v.height,
                    viewLeft:v.offsetLeft, viewRight:v.offsetLeft+v.width,
                    gap: options.length > 1 ? options[1].getBoundingClientRect().top-options[0].getBoundingClientRect().bottom : 0,
                    rowHeight:options[0]?.getBoundingClientRect().height,
                    popover:el.matches(':popover-open')};
            }""")
            assert data['popover'], data
            assert data['top'] >= data['viewTop'] + 7, data
            assert data['bottom'] <= data['viewBottom'] - 7, data
            assert data['left'] >= data['viewLeft'] + 7, data
            assert data['right'] <= data['viewRight'] - 7, data
            assert abs(data['gap']) <= 1 and 31 <= data['rowHeight'] <= 33, data
            await expect(field).to_be_focused()
            measurements.append(data)
            return menu

        async def exercise(field, query, kind):
            await page.evaluate('keyboardViewport(844)')
            # Pin the input for deterministic above/below distances; the actual
            # component and native popover remain mounted in the recipe form.
            await field.evaluate("el => { Object.assign(el.style, {position:'fixed', top:'260px', left:'20px', width:'280px', zIndex:'100'}); }")
            await field.fill(query)
            await geometry(field, 'below')
            await page.evaluate('keyboardViewport(400)')
            menu = await geometry(field, 'above')
            await field.press('ArrowUp')
            active = await field.get_attribute('aria-activedescendant')
            selected = page.locator(f'[id="{active}"]')
            await expect(selected).to_be_visible()
            await page.wait_for_function("""id => {
                const selected = document.getElementById(id);
                const list = selected?.closest('[role=listbox]');
                if (!selected || !list) return false;
                const row = selected.getBoundingClientRect(), menu = list.getBoundingClientRect();
                return row.top >= menu.top && row.bottom <= menu.bottom;
            }""", arg=active)
            selected_box, menu_box = await selected.bounding_box(), await menu.bounding_box()
            assert selected_box['y'] >= menu_box['y']
            assert selected_box['y'] + selected_box['height'] <= menu_box['y'] + menu_box['height']
            # Manual scrolling must not snap to the keyboard-highlighted row.
            await menu.evaluate('el => el.scrollTop = 0')
            await page.wait_for_timeout(80)
            assert await menu.evaluate('el => el.scrollTop') == 0
            await page.evaluate('keyboardViewport(350, 50)')
            await geometry(field, 'above')
            await page.screenshot(path=str(args.output / f'{kind}-keyboard.png'))
            await page.evaluate('keyboardViewport(844)')
            menu = await geometry(field, 'below')
            expected = await menu.get_by_role('option').first.inner_text()
            await menu.get_by_role('option').first.tap()
            await expect(field).to_have_attribute('aria-expanded', 'false')
            if kind != 'unit':
                await expect(field).to_have_value(expected)
            assert await page.locator('.ingredient-autocomplete__list:popover-open').count() == 0
            await field.evaluate("el => el.removeAttribute('style')")
            passed(f'{kind}: below/above flip, keyboard resize/pan, bounded height/width, compact rows, scrolling, touch and focus')

        try:
            await page.goto(base + '/login', wait_until='networkidle')
            await page.get_by_role('link', name='Continue with Google').click()
            await page.locator('input[name=username]').fill('viewport-' + suffix)
            await page.locator('textarea[name=claims]').fill(json.dumps({
                'email': f'viewport-{suffix}@example.test', 'email_verified': True, 'name': 'Viewport Test',
            }))
            await page.locator('input[type=submit]').click()
            await page.wait_for_url(base + '/**')
            await page.goto(base + '/recipes/new', wait_until='networkidle')
            await page.locator('.recipe-editor__contents a[href="#recipe-ingredients"]').click()
            await expect(page.locator('#recipe-ingredients')).to_be_focused()
            await page.get_by_role('button', name='+ Add Ingredient', exact=True).click()
            await exercise(page.get_by_role('combobox', name='Ingredient 1 name', exact=True), 'gar', 'ingredient')
            await exercise(page.get_by_role('combobox', name='Ingredient 1 unit', exact=True), '', 'unit')
            await page.locator('.recipe-editor__contents a[href="#recipe-equipment"]').click()
            await expect(page.locator('#recipe-equipment')).to_be_focused()
            await page.get_by_role('button', name='+ Add Equipment', exact=True).click()
            await exercise(page.get_by_role('combobox', name='Equipment 1 name', exact=True), 'pan', 'equipment')

            # A dialog provides a native modal top layer and a scrollable parent.
            await page.goto(base + '/kitchen/pantry/new', wait_until='networkidle')
            await page.get_by_label('Pantry name', exact=True).fill('Viewport pantry ' + suffix)
            add = page.locator('form.pantry-add')
            await add.get_by_role('combobox', name='Ingredient name', exact=True).fill('Rice')
            await add.get_by_label('Amount', exact=True).fill('1')
            await add.get_by_role('combobox', name='Unit', exact=True).fill('grams')
            await add.get_by_role('combobox', name='Unit', exact=True).press('Tab')
            await add.get_by_role('button', name='Add ingredient', exact=True).click()
            await page.get_by_role('button', name='Edit initial Rice', exact=True).click()
            dialog = page.get_by_role('dialog')
            unit = dialog.get_by_role('combobox', name='Unit', exact=True)
            await unit.focus()
            await unit.fill('')
            await expect(page.get_by_role('listbox', name='Unit suggestions')).to_be_visible()
            await page.evaluate('keyboardViewport(500)')
            ident = await unit.get_attribute('aria-controls')
            await page.wait_for_function("id => document.getElementById(id)?.matches(':popover-open')", arg=ident)
            menu = page.locator(f'[id="{ident}"]')
            await expect(menu).to_be_visible()
            input_bounds = await unit.bounding_box()
            assert input_bounds['y'] >= 8 and input_bounds['y'] + input_bounds['height'] <= 492, input_bounds
            await menu.get_by_role('option', name='gram', exact=True).tap()
            await expect(unit).to_have_value('g')
            await expect(dialog).to_be_visible()
            await expect(unit).to_be_focused()
            passed('native dialog: popover escapes clipping, remains interactive and preserves modal focus')

            await page.evaluate('keyboardViewport(844)')
            await dialog.get_by_role('button', name='Cancel', exact=True).click()
            assert await page.locator('.ingredient-autocomplete__list:popover-open').count() == 0
            # Older browsers use the resized layout viewport as the fallback.
            await page.evaluate("Object.defineProperty(window, 'visualViewport', {value:null, configurable:true})")
            await page.set_viewport_size({'width': 390, 'height': 420})
            await add.get_by_role('combobox', name='Unit', exact=True).focus()
            fallback = page.get_by_role('listbox', name='Unit suggestions')
            await expect(fallback).to_be_visible()
            bounds = await fallback.bounding_box()
            assert bounds['y'] >= 0 and bounds['y'] + bounds['height'] <= 420
            passed('layout viewport fallback, keyboard dismissal and dialog cleanup')
            assert not errors, errors
            (args.output / 'result.json').write_text(json.dumps(dict(checks=checks, errors=errors, measurements=measurements), indent=2))
        except Exception:
            print('PAGE ERRORS:', errors, flush=True)
            await page.screenshot(path=str(args.output / 'failure.png'))
            raise
        finally:
            await browser.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-url', required=True)
    parser.add_argument('--chromium', default='/usr/bin/chromium')
    parser.add_argument('--output', type=Path, default=Path('/tmp/noms-autocomplete-viewport-browser'))
    asyncio.run(main(parser.parse_args()))
