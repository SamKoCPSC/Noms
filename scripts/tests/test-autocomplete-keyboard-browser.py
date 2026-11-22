#!/usr/bin/env python3
"""Autocomplete placement against visual-only and resize-both mobile keyboards."""
import argparse
import asyncio
import json
import re
import uuid
from pathlib import Path
from urllib.parse import urlsplit

VIEWPORT = """(() => {
  const viewport = new EventTarget();
  let height = null, offset = 0;
  Object.defineProperties(viewport, {
    height: {get: () => height ?? window.innerHeight},
    width: {get: () => window.innerWidth},
    offsetTop: {get: () => offset}, offsetLeft: {get: () => 0},
    pageTop: {get: () => window.scrollY + offset},
    pageLeft: {get: () => window.scrollX}, scale: {get: () => 1}
  });
  Object.defineProperty(window, 'visualViewport', {configurable: true, get: () => viewport});
  window.editorKeyboard = (nextHeight = null, nextOffset = 0) => {
    height = nextHeight; offset = nextOffset;
    viewport.dispatchEvent(new Event('resize'));
    viewport.dispatchEvent(new Event('scroll'));
  };
})();"""

async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=30000)
    assert re.fullmatch(r'noms-[a-z0-9][a-z0-9-]*', args.instance)
    metadata = dict(line.split('=', 1) for line in
                    (Path('/tmp/noms-dev') / args.instance / 'metadata').read_text().splitlines() if '=' in line)
    base = metadata['APP_URL']
    assert metadata['STATE'] == 'ready' and urlsplit(base).hostname in {'127.0.0.1', 'localhost', '::1'}
    args.output.mkdir(parents=True, exist_ok=True)
    checks, errors = [], []
    async with async_playwright() as p:
        browser = await p.chromium.launch(executable_path='/usr/bin/chromium', args=['--disable-dev-shm-usage'])
        context = await browser.new_context(viewport={'width': 390, 'height': 844}, is_mobile=True, has_touch=True)
        await context.add_init_script(VIEWPORT)
        await context.route(re.compile(r'https://fonts\.(googleapis|gstatic)\.com/'), lambda route: route.abort())
        page = await context.new_page()
        page.set_default_navigation_timeout(120000)
        page.on('pageerror', lambda error: errors.append(str(error)))

        async def frames():
            await page.evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))')

        async def keyboard(height=None, offset=0):
            await page.evaluate('([height,offset])=>editorKeyboard(height,offset)', [height, offset])
            await expect(page.locator('html')).to_have_attribute('data-mobile-keyboard', 'closed' if height is None else 'open')
            await frames()

        async def place(field, top):
            await field.evaluate('(el,top)=>window.scrollBy({top:el.getBoundingClientRect().top-top,behavior:"instant"})', top)
            await frames()

        async def menu_for(field):
            return page.locator('[id="' + await field.get_attribute('aria-controls') + '"]')

        async def fits(field, height=500, offset=0):
            menu = await menu_for(field)
            await expect(menu).to_be_visible()
            assert await menu.evaluate("el=>el.matches(':popover-open')")
            await expect(menu).to_have_attribute('data-placement', 'below')
            await expect(field).to_be_focused()
            await frames()
            box = await menu.bounding_box()
            field_box = await field.bounding_box()
            assert box['y'] >= field_box['y'] + field_box['height'] - 1
            assert box['y'] + box['height'] <= offset + height - 6, box
            assert box['height'] >= 44, box
            assert field_box['y'] >= offset
            return menu

        suffix = uuid.uuid4().hex[:10]
        await page.goto(base + '/login', wait_until='networkidle')
        await page.get_by_role('link', name='Continue with Google').click()
        await page.locator('input[name=username]').fill('autocomplete-keyboard-' + suffix)
        await page.locator('textarea[name=claims]').fill(json.dumps({
            'email': f'autocomplete-keyboard-{suffix}@example.test', 'email_verified': True, 'name': 'Autocomplete Keyboard Test'}))
        await page.locator('input[type=submit]').click()
        await page.wait_for_url(base + '/**')
        await page.goto(base + '/recipes/new', wait_until='networkidle')
        await page.get_by_label('Recipe name', exact=True).fill('Keyboard viewport ' + suffix)
        await page.get_by_label('Servings', exact=True).fill('2')
        await page.get_by_role('button', name='+ Add Ingredient', exact=True).click()
        name = page.get_by_label('Ingredient 1 name', exact=True)
        unit = page.get_by_label('Ingredient 1 unit', exact=True)
        await name.fill('salt')
        primary = page.locator('.recipe-ingredient-editor').first
        await name.press('Escape')
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        await primary.get_by_role('button', name='Add substitution', exact=True).click()
        alternative = page.get_by_label('Ingredient 1 substitution 1 name', exact=True)
        await alternative.fill('salt')
        await alternative.press('Escape')
        await page.get_by_role('button', name='+ Add Equipment', exact=True).click()
        equipment = page.get_by_role('combobox', name='Equipment 1 name', exact=True)
        await equipment.fill('pan')
        await equipment.press('Escape')

        await name.focus()
        await name.fill('salt')
        await expect((await menu_for(name)).get_by_role('option').first).to_be_visible()
        await place(name, 400)
        before = await page.evaluate('scrollY')
        await keyboard(500)
        assert await page.evaluate('scrollY') > before + 100
        assert (await name.bounding_box())['y'] <= 110
        menu = await fits(name)
        # Compact rows may all fit at 500px; use a shorter viewport to exercise scrolling.
        if not await menu.evaluate('el=>el.scrollHeight>el.clientHeight'):
            await keyboard(350)
            menu = await fits(name, height=350)
        assert await menu.evaluate('el=>el.scrollHeight>el.clientHeight')
        stable = await page.evaluate('scrollY')
        await menu.evaluate('el=>el.scrollTop=el.scrollHeight')
        await frames()
        assert abs(await page.evaluate('scrollY') - stable) <= 1
        await keyboard(500)
        stable = await page.evaluate('scrollY')
        await name.press('End')
        await name.press('Backspace')
        await name.press('t')
        await fits(name)
        # Allow native caret-reveal rounding while rejecting page repositioning.
        after_typing = await page.evaluate('scrollY')
        if abs(after_typing - stable) > 3:
            print('Typing scroll diagnostics:', {'before': stable, 'after': after_typing,
                'field': await name.bounding_box(), 'menu': await menu.bounding_box(),
                'scrollers': await page.locator('[data-autocomplete-scroller=true]').evaluate_all('els=>els.map(el=>({tag:el.tagName,class:el.className,top:el.scrollTop}))')}, flush=True)
            await page.screenshot(path=str(args.output / 'typing-failure.png'))
        assert abs(after_typing - stable) <= 3, 'Typing must not reposition the page'
        await page.evaluate('window.scrollBy({top:-16,behavior:"instant"})')
        await frames()
        manual = await page.evaluate('scrollY')
        await name.press('End')
        await name.press('Backspace')
        await name.press('t')
        await fits(name)
        assert abs(await page.evaluate('scrollY') - manual) <= 3, 'Manual scrolling of a visible field must not be undone'
        checks.append('ingredient input clears overlay keyboard; list fits, scrolls independently, and typing/manual scroll stay steady')
        print('PASS:', checks[-1], flush=True)

        # The keyboard stays open as focus changes to each autocomplete kind.
        for field, query in [(unit, ''), (alternative, 'salt'), (equipment, 'pan')]:
            await place(field, 400)
            await field.focus()
            await field.fill(query)
            menu = await fits(field)
            assert (await field.bounding_box())['y'] <= 110
            stable = await page.evaluate('scrollY')
            await field.press('ArrowUp')
            await frames()
            active_id = await field.get_attribute('aria-activedescendant')
            assert active_id
            selected = page.locator('[id="' + active_id + '"]')
            box, menu_box = await selected.bounding_box(), await menu.bounding_box()
            assert box['y'] >= menu_box['y'] - 1
            assert box['y'] + box['height'] <= menu_box['y'] + menu_box['height'] + 1
            assert abs(await page.evaluate('scrollY') - stable) <= 1
            await field.press('Enter')
            await expect(field).to_have_attribute('aria-expanded', 'false')
        checks.append('unit, substitution and equipment focus transitions with keyboard open; arrow navigation stays inside the dropdown')
        print('PASS:', checks[-1], flush=True)

        await keyboard()
        await name.focus()
        await name.fill('salt')
        await place(name, 100)
        stable = await page.evaluate('scrollY')
        await keyboard(500)
        await fits(name)
        assert abs(await page.evaluate('scrollY') - stable) <= 1, 'An input with enough room must stay put'
        await keyboard(350)
        await fits(name, height=350)
        assert (await name.bounding_box())['y'] <= 110
        await keyboard()
        await place(name, 400)
        await page.set_viewport_size({'width': 390, 'height': 500})
        await keyboard(500)
        menu = await fits(name)
        await page.screenshot(path=str(args.output / 'autocomplete-keyboard-mobile.png'))
        chosen = (await menu.get_by_role('option').first.inner_text()).strip()
        await menu.get_by_role('option').first.tap()
        await expect(name).to_have_value(chosen)
        await expect(name).to_have_attribute('aria-expanded', 'false')
        checks.append('already-visible inputs remain stable; keyboard animation, resize-both Android layout and touch selection work')
        print('PASS:', checks[-1], flush=True)

        await page.set_viewport_size({'width': 390, 'height': 844})
        await keyboard()
        await expect(page.locator('html')).not_to_have_attribute('data-mobile-autocomplete', 'open')
        await page.set_viewport_size({'width': 1440, 'height': 1000})
        await name.fill('salt')
        await page.evaluate('editorKeyboard(500)')
        await expect(page.locator('html')).to_have_attribute('data-mobile-keyboard', 'closed')
        await expect(page.locator('html')).not_to_have_attribute('data-mobile-autocomplete', 'open')
        await expect(await menu_for(name)).to_have_css('max-height', '192px')
        await page.get_by_label('Recipe name', exact=True).focus()
        checks.append('keyboard close cleans up scroll room; shared popovers retain the compact desktop height')
        print('PASS:', checks[-1], flush=True)

        await page.set_viewport_size({'width': 390, 'height': 844})
        await keyboard()
        await page.goto(base + '/kitchen/pantry/new', wait_until='networkidle')
        await page.get_by_label('Pantry name', exact=True).fill('Keyboard pantry ' + suffix)
        add = page.locator('form.pantry-add')
        pantry_name = add.get_by_role('combobox', name='Ingredient name', exact=True)
        await add.get_by_role('combobox', name='Unit', exact=True).fill('g')
        await add.get_by_role('combobox', name='Unit', exact=True).press('Enter')
        await pantry_name.fill('salt')
        await keyboard(500)
        menu = await fits(pantry_name)
        pantry_chosen = (await menu.get_by_role('option').first.inner_text()).strip()
        await menu.get_by_role('option').first.tap()
        await keyboard()
        await add.get_by_role('button', name='Add ingredient', exact=True).click()
        await expect(page.locator('.pantry-draft-row')).to_have_count(1)
        await page.get_by_role('button', name='Edit initial ' + pantry_chosen, exact=True).click()
        dialog = page.get_by_role('dialog')
        edited = dialog.get_by_role('combobox', name='Ingredient name', exact=True)
        await edited.fill('salt')
        await keyboard(500)
        menu = await fits(edited)
        box = await dialog.bounding_box()
        assert box['y'] >= 0 and box['y'] + box['height'] <= 501
        assert await dialog.locator('[data-autocomplete-scroller=true]').count() == 1
        await page.screenshot(path=str(args.output / 'autocomplete-keyboard-pantry-dialog.png'), clip={'x': 0, 'y': 0, 'width': 390, 'height': 500})
        chosen = (await menu.get_by_role('option').first.inner_text()).strip()
        await menu.get_by_role('option').first.tap()
        await expect(edited).to_have_value(chosen)
        await expect(dialog).to_be_visible()
        await keyboard()
        assert await page.locator('[data-autocomplete-scroller=true]').count() == 0
        await dialog.get_by_role('button', name='Cancel', exact=True).click()
        pantry_unit = add.get_by_role('combobox', name='Unit', exact=True)
        await pantry_unit.focus()
        await pantry_unit.fill('')
        await keyboard(500)
        await fits(pantry_unit)
        await keyboard()
        checks.append('pantry quick-add and modal autocomplete fit the keyboard viewport, retain touch selection and clean up scroll room')
        print('PASS:', checks[-1], flush=True)
        await browser.close()
    assert not errors, errors
    result = {'checks': checks, 'javascript_errors': errors}
    (args.output / 'result.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--instance', required=True)
    parser.add_argument('--output', type=Path, required=True)
    asyncio.run(main(parser.parse_args()))
