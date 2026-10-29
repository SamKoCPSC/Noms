#!/usr/bin/env python3
"""Mobile authoring and keyboard viewport regressions in a local mock-auth stack."""
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
    meta = dict(line.split('=', 1) for line in
                (Path('/tmp/noms-dev') / args.instance / 'metadata').read_text().splitlines() if '=' in line)
    base = meta['APP_URL']
    assert meta['STATE'] == 'ready' and urlsplit(base).hostname in {'127.0.0.1', 'localhost', '::1'}
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
        suffix = uuid.uuid4().hex[:10]
        await page.goto(base + '/login', wait_until='networkidle')
        await page.get_by_role('link', name='Continue with Google').click()
        await page.locator('input[name=username]').fill('mobile-editor-' + suffix)
        await page.locator('textarea[name=claims]').fill(json.dumps({
            'email': f'mobile-editor-{suffix}@example.test', 'email_verified': True, 'name': 'Mobile Editor Demo'}))
        await page.locator('input[type=submit]').click()
        await page.wait_for_url(base + '/**')
        await page.goto(base + '/recipes/new', wait_until='networkidle')
        await page.get_by_label('Recipe name', exact=True).fill('Mobile recipe ' + suffix)
        await page.get_by_label('Servings', exact=True).fill('2')
        sections = page.get_by_role('navigation', name='Recipe editor sections', exact=True)
        await sections.get_by_role('link', name='Ingredients', exact=False).click()
        await page.get_by_role('button', name='+ Add Ingredient', exact=True).click()
        name = page.get_by_label('Ingredient 1 name', exact=True)
        await name.fill('Chickpeas')
        await name.press('Escape')
        await page.get_by_label('Ingredient 1 amount', exact=True).fill('200')
        await page.get_by_label('Ingredient 1 unit', exact=True).fill('g')
        await page.get_by_label('Ingredient 1 unit', exact=True).press('Enter')
        primary = page.locator('.recipe-ingredient-editor').first
        await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await page.get_by_label('Ingredient 1 text before', exact=True).fill('Acme')
        await page.get_by_label('Ingredient 1 text after', exact=True).fill(', drained and rinsed')
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        await primary.get_by_role('button', name='Add substitution', exact=True).click()
        await page.get_by_label('Ingredient 1 substitution 1 name', exact=True).fill('White beans')
        await page.get_by_label('Ingredient 1 substitution 1 name', exact=True).press('Escape')
        await page.get_by_label('Ingredient 1 substitution 1 amount', exact=True).fill('200')
        await page.get_by_label('Ingredient 1 substitution 1 unit', exact=True).fill('g')
        await page.get_by_label('Ingredient 1 substitution 1 unit', exact=True).press('Enter')

        async def keyboard(opened):
            await expect(page.locator('html')).to_have_attribute('data-mobile-keyboard', 'open' if opened else 'closed')
            toolbar = page.locator('.recipe-editor__actions')
            if opened:
                await expect(toolbar).to_be_hidden()
            else:
                await expect(toolbar).to_be_visible()

        async def check_toolbar(button_count):
            toolbar = page.locator('.recipe-editor__actions')
            await expect(toolbar.get_by_role('button')).to_have_count(button_count)
            assert (await toolbar.bounding_box())['height'] <= 64
            await expect(toolbar).to_have_css('box-shadow', 'none')
            for button in await toolbar.get_by_role('button').all():
                assert '\n' not in (await button.inner_text()).strip()
                await expect(button).to_have_css('box-shadow', 'none')
                assert 'system-ui' in await button.evaluate('el=>getComputedStyle(el).fontFamily')

        for width in [320, 390, 430, 760]:
            await page.set_viewport_size({'width': width, 'height': 844})
            await page.evaluate('editorKeyboard()')
            await keyboard(False)
            for prefix in ['Ingredient 1', 'Ingredient 1 substitution 1']:
                field = page.get_by_label(prefix + ' name', exact=True)
                basic = field.locator('xpath=ancestor::div[contains(@class,"recipe-ingredient-basic")]')
                field_box, basic_box = await field.bounding_box(), await basic.bounding_box()
                assert abs(field_box['width'] - basic_box['width']) <= 1, (width, prefix)
                quantity = [await basic.get_by_label(prefix + ' ' + part, exact=True).bounding_box() for part in ['amount', 'unit', 'optional']]
                centers = [box['y'] + box['height']/2 for box in quantity]
                assert max(centers) - min(centers) <= 8
                modifier = await basic.get_by_role('button', name=prefix + ' modifiers', exact=True).bounding_box()
                assert modifier['y'] >= max(box['y'] + box['height'] for box in quantity)
                remove = await basic.get_by_role('button', name='Remove ' + prefix, exact=True).bounding_box()
                assert remove['y'] + remove['height'] <= field_box['y']
            assert (await sections.bounding_box())['height'] <= 52
            tab_boxes = [await link.bounding_box() for link in await sections.get_by_role('link').all()]
            assert max(box['y'] for box in tab_boxes) - min(box['y'] for box in tab_boxes) <= 1
            assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
            await check_toolbar(3)
        checks.append('full-width primary/substitution names, quantity alignment, modifiers below quantity, single-row sections and flat toolbar at 320–760px')
        print('PASS:', checks[-1], flush=True)

        await page.set_viewport_size({'width': 390, 'height': 844})
        await page.evaluate('editorKeyboard()')
        await name.focus()
        await name.press('Escape')
        await keyboard(False)  # Hardware keyboard or ordinary focus must keep actions.
        await page.evaluate('editorKeyboard(760)')
        await keyboard(False)  # Browser chrome is not an onscreen keyboard.
        await page.evaluate('editorKeyboard(500, 20)')
        await keyboard(True)
        await expect(page.locator('.recipe-editor__contents')).to_have_css('position', 'static')
        await page.evaluate('editorKeyboard()')
        await keyboard(False)  # Closing the keyboard does not require blurring.
        await page.evaluate('editorKeyboard(500)')
        await page.set_viewport_size({'width': 390, 'height': 500})
        await keyboard(True)  # Regression: Android resizes BOTH viewports.
        await name.fill('Chickpeas, cooked')
        await name.press('Escape')
        await name.evaluate('el=>el.scrollIntoView({block:"center",behavior:"instant"})')
        await page.screenshot(path=str(args.output / 'keyboard-open.png'))
        await page.set_viewport_size({'width': 390, 'height': 844})
        await page.evaluate('editorKeyboard()')
        await keyboard(False)
        await expect(name).to_have_value('Chickpeas, cooked')
        await page.get_by_label('Ingredient 1 optional', exact=True).focus()
        await page.evaluate('editorKeyboard(500)')
        await keyboard(False)  # Checkboxes never imply a text keyboard.
        await page.evaluate('editorKeyboard()')
        checks.append('keyboard open/close, viewport panning, resize-both Android behavior, hardware focus, browser chrome, and checkbox focus')
        print('PASS:', checks[-1], flush=True)

        # Last tabs remain reachable without remounting the autosaved draft.
        retained_name = await name.element_handle()
        await sections.get_by_role('link', name='Notes', exact=False).click()
        await expect(page.locator('.recipe-editor__notes')).to_be_visible()
        await sections.get_by_role('link', name='Ingredients', exact=False).click()
        await expect(name).to_have_value('Chickpeas, cooked')
        assert await retained_name.evaluate('el=>el.isConnected'), 'Section switching must retain the input DOM'
        await expect(page.get_by_label('Ingredient 1 text before', exact=True)).to_have_value('Acme')
        await expect(page.get_by_role('button', name='Saved', exact=True)).to_be_visible()
        await page.reload(wait_until='networkidle')
        await sections.get_by_role('link', name='Ingredients', exact=False).click()
        await primary.get_by_role('button', name='Edit Ingredient 1', exact=True).click()
        await name.press('Escape')
        await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await expect(page.get_by_label('Ingredient 1 text after', exact=True)).to_have_value(', drained and rinsed')
        await check_toolbar(4)
        await page.get_by_role('button', name='Review', exact=True).click()
        dialog = page.get_by_role('dialog')
        await expect(dialog.get_by_role('heading', name='Review your recipe', exact=True)).to_be_visible()
        await dialog.get_by_role('button', name='Keep editing', exact=True).click()
        await expect(dialog).to_have_count(0)
        await keyboard(False)
        checks.append('last section navigation, retained field values, saved draft reload, four-action toolbar and publication review')
        print('PASS:', checks[-1], flush=True)
        await sections.get_by_role('link', name='Ingredients', exact=False).click()
        await page.screenshot(path=str(args.output / 'editor-mobile.png'))

        await page.set_viewport_size({'width': 1440, 'height': 1000})
        await keyboard(False)
        await expect(page.locator('.recipe-editor__actions')).to_have_css('position', 'static')
        await expect(page.get_by_role('button', name='Publish Changes', exact=True)).to_be_visible()
        await page.get_by_role('button', name='Toggle theme', exact=True).click()
        await page.set_viewport_size({'width': 390, 'height': 844})
        await expect(page.locator('html')).to_have_class(re.compile('dark'))
        await check_toolbar(4)
        await page.screenshot(path=str(args.output / 'editor-mobile-dark.png'))
        checks.append('desktop actions preserved and mobile dark theme remains flat and compact')
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
