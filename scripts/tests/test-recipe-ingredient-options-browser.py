#!/usr/bin/env python3
"""Ingredient authoring against a ready local mock-auth stack. Cookies stay in memory."""
import argparse
import asyncio
import json
import re
import subprocess
import uuid
from pathlib import Path
from urllib.parse import urlsplit


def sql(instance, statement):
    return subprocess.run(
        ['docker', '--host=unix:///var/run/docker.sock', 'exec', '-i', f'{instance}-postgres-1',
         'sh', '-c', 'psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v ON_ERROR_STOP=1 -At'],
        input=statement, capture_output=True, text=True, check=True).stdout.strip()


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
        context = await browser.new_context(viewport={'width': 1440, 'height': 1100})
        await context.route(re.compile(r'https://fonts\.(googleapis|gstatic)\.com/'), lambda route: route.abort())
        page = await context.new_page()
        page.set_default_navigation_timeout(120000)
        page.on('pageerror', lambda error: errors.append(str(error)))
        suffix = uuid.uuid4().hex[:10]
        await page.goto(base + '/login', wait_until='networkidle')
        await page.get_by_role('link', name='Continue with Google').click()
        await page.locator('input[name=username]').fill('ingredient-options-' + suffix)
        await page.locator('textarea[name=claims]').fill(json.dumps({
            'email': f'ingredient-options-{suffix}@example.test', 'email_verified': True, 'name': 'Ingredient Demo'}))
        await page.locator('input[type=submit]').click()
        await page.wait_for_url(base + '/**')
        await page.goto(base + '/recipes/new', wait_until='networkidle')
        await page.get_by_label('Recipe name', exact=True).fill('Ingredient options ' + suffix)
        await page.get_by_label('Servings', exact=True).fill('2')
        await page.get_by_role('button', name='+ Add Ingredient', exact=True).click()
        await expect(page.locator('.recipe-ingredient-basic').first).to_have_css('display','grid')
        await page.get_by_label('Ingredient 1 name', exact=True).fill('chickpea')
        field = page.get_by_label('Ingredient 1 name', exact=True)
        await expect(field).to_have_attribute('aria-expanded', 'true')
        options = page.locator('#recipe-ingredient-0-name-suggestions').get_by_role('option')
        await options.first.click()
        primary_name = await field.input_value()
        await page.get_by_label('Ingredient 1 amount', exact=True).fill('2')
        await page.get_by_label('Ingredient 1 unit', exact=True).fill('grammes')
        await page.get_by_label('Ingredient 1 unit', exact=True).press('Enter')
        await expect(page.get_by_label('Ingredient 1 unit', exact=True)).to_have_value('g')
        primary = page.locator('.recipe-ingredient-editor').first
        await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await page.get_by_label('Ingredient 1 optional', exact=True).check()
        await page.get_by_label('Ingredient 1 text before', exact=True).fill('Acme')
        await page.get_by_label('Ingredient 1 text after', exact=True).fill(', drained and rinsed')
        await expect(primary.get_by_text('add extra details about the ingredient',exact=True)).to_be_visible()
        await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await expect(page.get_by_label('Ingredient 1 text before',exact=True)).to_be_hidden()
        await expect(primary.locator('.recipe-ingredient-preview').first).to_be_visible()
        toggle=primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True)
        await expect(toggle).to_have_attribute('aria-expanded','false')
        await toggle.focus()
        await toggle.press('Enter')
        await expect(toggle).to_have_attribute('aria-expanded','true')
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        await expect(primary.get_by_text('add alternative ingredients',exact=True)).to_be_visible()
        await primary.get_by_role('button', name='Add substitution', exact=True).click()
        await page.get_by_label('Ingredient 1 substitution 1 name', exact=True).fill('lentil')
        await expect(page.get_by_label('Ingredient 1 substitution 1 name', exact=True)).to_have_attribute('aria-expanded', 'true')
        await page.locator('#recipe-ingredient-0-substitute-0-name-suggestions').get_by_role('option').first.click()
        substitute_name = await page.get_by_label('Ingredient 1 substitution 1 name', exact=True).input_value()
        await page.get_by_label('Ingredient 1 substitution 1 amount', exact=True).fill('400')
        await page.get_by_label('Ingredient 1 substitution 1 unit', exact=True).fill('g')
        await page.get_by_label('Ingredient 1 substitution 1 unit', exact=True).press('Enter')
        substitute = primary.locator('.recipe-ingredient-substitute').first
        await substitute.get_by_role('button', name='Ingredient 1 substitution 1 modifiers', exact=True).click()
        await page.get_by_label('Ingredient 1 substitution 1 optional', exact=True).check()
        await page.get_by_label('Ingredient 1 substitution 1 text before', exact=True).fill('cooked')
        await page.get_by_label('Ingredient 1 substitution 1 text after', exact=True).fill('and cooled')
        await primary.get_by_role('button', name='Add substitution', exact=True).click()
        await page.get_by_label('Ingredient 1 substitution 2 name', exact=True).fill('Grandma’s special beans')
        await page.get_by_role('button', name='Remove Ingredient 1 substitution 2', exact=True).click()
        await expect(primary.locator('.recipe-ingredient-substitute')).to_have_count(1)
        await expect(primary.locator('.recipe-ingredient-preview').first).to_have_text(f'2 g Acme {primary_name}, drained and rinsed (optional)')
        checks.append('primary and substitute autocomplete, independent fields, optional flags, preview, add/remove')
        print('PASS:',checks[-1],flush=True)
        await page.get_by_label('Ingredient 1 text before', exact=True).fill('Brand' * 100)
        for width in [320,390,760,1100,1440]:
            await page.set_viewport_size({'width':width,'height':1100})
            await expect(primary).to_be_visible()
            assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth'), f'Overflow at {width}'
            basic=primary.locator(':scope > .recipe-ingredient-basic')
            boxes=[await basic.get_by_label('Ingredient 1 '+name,exact=True).bounding_box() for name in ['amount','unit','optional']]
            centers=[box['y']+box['height']/2 for box in boxes]
            assert max(centers)-min(centers)<=8, f'Optional is not beside quantity at {width}'
            inline_names=['name','modifiers'] + (['amount','unit','optional'] if width>760 else [])
            inline_boxes=[]
            for name in inline_names:
                control=basic.get_by_role('button',name='Ingredient 1 modifiers',exact=True) if name=='modifiers' else basic.get_by_label('Ingredient 1 '+name,exact=True)
                inline_boxes.append(await control.bounding_box())
            inline_centers=[box['y']+box['height']/2 for box in inline_boxes]
            assert max(inline_centers)-min(inline_centers)<=8, f'Modifiers is not inline at {width}'
            assert all(box['width']>=44 for box in boxes[:2]), f'Quantity inputs too narrow at {width}'
            for row in [primary, primary.locator('.recipe-ingredient-substitute').first]:
                assert await row.evaluate('el=>el.scrollWidth<=el.clientWidth'), f'Ingredient row overflow at {width}'
            order=await primary.evaluate("""el=>{const mods=el.querySelector(':scope > .recipe-ingredient-modifier-section'), preview=el.querySelector(':scope > .recipe-ingredient-preview'), subs=el.querySelector(':scope > .recipe-ingredient-substitutions');return !!(mods.compareDocumentPosition(preview)&Node.DOCUMENT_POSITION_FOLLOWING)&&!!(preview.compareDocumentPosition(subs)&Node.DOCUMENT_POSITION_FOLLOWING)}""")
            assert order, 'Modifiers, display text and substitutions must stay in order'
        await page.set_viewport_size({'width':1440,'height':1100})
        if await primary.get_by_role('button',name='Ingredient 1 modifiers',exact=True).get_attribute('aria-expanded') != 'true':
            await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await page.get_by_label('Ingredient 1 text before', exact=True).fill('Acme')
        await primary.get_by_role('button',name='Ingredient 1 modifiers',exact=True).click()
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        for width,filename in [(1440,'collapsed-desktop.png'),(390,'collapsed-mobile.png')]:
            await page.set_viewport_size({'width':width,'height':2000})
            await page.evaluate('window.scrollTo({top:0,behavior:"instant"})')
            await expect(page.get_by_label('Ingredient 1 text before',exact=True)).to_be_hidden()
            await expect(primary.get_by_text('add alternative ingredients',exact=True)).to_be_hidden()
            await expect(primary.locator(':scope > .recipe-ingredient-preview')).to_be_visible()
            await primary.screenshot(path=str(args.output/filename))
        await primary.get_by_role('button',name='Ingredient 1 modifiers',exact=True).click()
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        await page.set_viewport_size({'width':1440,'height':1100})
        await primary.screenshot(path=str(args.output/'editor-desktop.png'))
        await page.set_viewport_size({'width':390,'height':2000})
        await page.evaluate('window.scrollTo({top:0,behavior:"instant"})')
        if await primary.get_by_role('button',name='Ingredient 1 modifiers',exact=True).get_attribute('aria-expanded') != 'true':
            await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await primary.screenshot(path=str(args.output/'editor-mobile.png'))
        await page.set_viewport_size({'width':1440,'height':1100})
        await page.get_by_role('button',name='Toggle theme',exact=True).click()
        await expect(page.locator('html')).to_have_class(re.compile('dark'))
        await primary.screenshot(path=str(args.output/'editor-dark.png'))
        await page.set_viewport_size({'width':320,'height':1000})
        assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
        await page.set_viewport_size({'width':1440,'height':1100})
        await page.get_by_role('button',name='Toggle theme',exact=True).click()
        checks.append('320/390/760/1100/1440px layouts and dark appearance')
        print('PASS:',checks[-1],flush=True)
        await page.get_by_role('button', name='+ Add Step', exact=True).click()
        await page.get_by_label('Method step 1', exact=True).fill('Warm the beans and serve.')
        # Wait for autosave before reloading.
        await expect(page.get_by_role('button',name='Saved',exact=True)).to_be_visible()
        await page.reload(wait_until='networkidle')
        await expect(page.get_by_label('Ingredient 1 name',exact=True)).to_have_value(primary_name)
        primary = page.locator('.recipe-ingredient-editor').first
        await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await expect(page.get_by_label('Ingredient 1 optional',exact=True)).to_be_checked()
        await expect(page.get_by_label('Ingredient 1 text before',exact=True)).to_have_value('Acme')
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        await expect(primary.get_by_text('add alternative ingredients',exact=True)).to_be_visible()
        await expect(page.get_by_label('Ingredient 1 substitution 1 amount',exact=True)).to_have_value('400')
        await primary.get_by_role('button', name='Ingredient 1 substitution 1 modifiers', exact=True).click()
        await expect(page.get_by_label('Ingredient 1 substitution 1 optional',exact=True)).to_be_checked()
        await expect(page.get_by_label('Ingredient 1 substitution 1 text after',exact=True)).to_have_value('and cooled')
        checks.append('private draft reload retains all ingredient and substitute fields')
        await page.get_by_role('button',name=re.compile(r'Publish (Recipe|Changes)')).click()
        await page.wait_for_url(re.compile(r'.*/recipes/[0-9a-f-]{36}$'))
        recipe_id = str(uuid.UUID(page.url.rsplit('/',1)[1]))
        stored=json.loads(sql(args.instance,f"SELECT ingredients FROM recipes WHERE id='{recipe_id}';"))
        assert stored[0]['optional'] is True and stored[0]['text_before']=='Acme'
        assert stored[0]['vocabulary_id'] and stored[0]['substitutes'][0]['vocabulary_id']
        assert stored[0]['substitutes'][0]['amount']=='400' and stored[0]['substitutes'][0]['unit']=='g'
        assert stored[0]['substitutes'][0]['optional'] is True
        await expect(page.locator('.recipe-detail__ingredients-section')).to_contain_text('Acme ' + primary_name + ', drained and rinsed (optional)')
        await expect(page.locator('.recipe-detail__ingredients-section')).to_contain_text('Or 400 g cooked ' + substitute_name + ' and cooled (optional)')
        checks.append('publication persists full JSON and shows modifiers, optional flags, and alternatives')
        await page.goto(base+f'/recipes/{recipe_id}/edit',wait_until='networkidle')
        await expect(page.get_by_label('Ingredient 1 name',exact=True)).to_have_value(primary_name)
        primary=page.locator('.recipe-ingredient-editor').first
        await primary.get_by_role('button', name='Ingredient 1 modifiers', exact=True).click()
        await page.get_by_label('Ingredient 1 optional',exact=True).uncheck()
        await page.get_by_label('Ingredient 1 text after',exact=True).fill(', warmed')
        await primary.locator(':scope > .recipe-ingredient-substitutions > summary').click()
        await expect(primary.get_by_text('add alternative ingredients',exact=True)).to_be_visible()
        await page.get_by_label('Ingredient 1 substitution 1 amount',exact=True).fill('500')
        await page.get_by_label('Ingredient 1 substitution 1 name',exact=True).fill('Grandma’s bean mix')
        await expect(page.get_by_role('button',name='Saved',exact=True)).to_be_visible()
        await page.get_by_role('button',name='Publish Changes',exact=False).click()
        await page.wait_for_url(base+f'/recipes/{recipe_id}')
        stored=json.loads(sql(args.instance,f"SELECT ingredients FROM recipes WHERE id='{recipe_id}';"))
        assert not stored[0].get('optional',False)
        assert stored[0]['text_after']==', warmed' and stored[0]['substitutes'][0]['amount']=='500'
        assert 'vocabulary_id' not in stored[0]['substitutes'][0] and stored[0]['vocabulary_id']
        versions=json.loads(sql(args.instance,f"SELECT json_agg(snapshot ORDER BY version_number) FROM recipe_versions WHERE recipe_id='{recipe_id}';"))
        assert versions[0]['ingredients'][0]['optional'] is True
        assert versions[0]['ingredients'][0]['substitutes'][0]['amount']=='400'
        assert versions[-1]['ingredients']==stored
        checks.append('edit publication, free-text identity clearing, and immutable history preserve independent values')
        await page.set_viewport_size({'width':390,'height':1000})
        await expect(page.locator('.mobile-scaling')).to_contain_text('Or 500 g cooked Grandma’s bean mix and cooled (optional)')
        await page.locator('.mobile-scaling').screenshot(path=str(args.output/'detail-mobile.png'))
        await page.get_by_role('button',name='Increase servings',exact=True).click()
        await expect(page.locator('.mobile-scaling')).to_contain_text('Or 750 g cooked Grandma’s bean mix and cooled (optional)')
        checks.append('mobile recipe view and independent substitute quantity scaling')
        await browser.close()
    assert not errors,errors
    result={'checks':checks,'javascript_errors':errors}
    (args.output/'result.json').write_text(json.dumps(result,indent=2))
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--instance',required=True)
    parser.add_argument('--output',type=Path,required=True)
    asyncio.run(main(parser.parse_args()))
