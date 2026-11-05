#!/usr/bin/env python3
"""Put-away regression; local mock-auth development stack only."""
import argparse
import asyncio
import importlib.util
import json
import re
import uuid
from pathlib import Path
from urllib.parse import urlsplit
spec=importlib.util.spec_from_file_location('groceries',Path(__file__).with_name('test-grocery-list-browser.py'))
helpers=importlib.util.module_from_spec(spec)
spec.loader.exec_module(helpers)

async def main(args):
    from playwright.async_api import async_playwright, expect
    expect.set_options(timeout=30000)
    assert re.fullmatch(r'noms-[a-z0-9][a-z0-9-]*',args.instance)
    meta=dict(line.split('=',1) for line in (Path('/tmp/noms-dev')/args.instance/'metadata').read_text().splitlines() if '=' in line)
    base=meta['APP_URL']; assert meta['STATE']=='ready' and urlsplit(base).hostname in {'127.0.0.1','localhost'}
    args.output.mkdir(parents=True,exist_ok=True)
    errors=[]; paths={}
    sql=lambda statement:helpers.sql(args.instance,statement)
    async with async_playwright() as p:
        browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--disable-dev-shm-usage'])
        ctx=await browser.new_context(viewport={'width':1440,'height':1000})
        await ctx.route(re.compile(r'https://fonts\.(googleapis|gstatic)\.com/'),lambda route:route.abort())
        page=await ctx.new_page();page.set_default_navigation_timeout(120000);page.on('pageerror',lambda e:errors.append(str(e)))
        await page.goto(base+'/login',wait_until='networkidle')
        await page.get_by_role('link',name='Continue with Google').click()
        assert urlsplit(page.url).hostname=='127.0.0.1' and str(urlsplit(page.url).port)==meta['MOCK_OAUTH_PORT']
        suffix=uuid.uuid4().hex[:10]
        await page.locator('input[name=username]').fill('put-away-'+suffix)
        await page.locator('textarea[name=claims]').fill(json.dumps({'email':f'putaway-{suffix}@example.test','email_verified':True,'name':'Put away '+suffix}))
        await page.locator('input[type=submit]').click();await page.wait_for_url(base+'/**')
        owner=str(uuid.UUID((await (await ctx.request.get(base+'/api/user_profile')).json())['current_user_id']))
        pantry,list_id,rice,beans,soap,unchecked,other,foreign=[str(uuid.uuid4()) for _ in range(8)]
        sql(f"""
INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash) VALUES('{pantry}','{owner}','Home pantry','{uuid.uuid4()}','{'a'*64}');
INSERT INTO pantry_ingredients(profile_id,name,amount,unit,available,position) VALUES('{pantry}','RICE','2','kg',FALSE,0);
INSERT INTO grocery_lists(id,owner_id,name,request_id,request_hash) VALUES('{list_id}','{owner}','Shopping','{uuid.uuid4()}','{'b'*64}');
INSERT INTO grocery_list_items(id,list_id,name,amount,unit,checked,position) VALUES('{rice}','{list_id}','Rice','500','g',TRUE,0),('{beans}','{list_id}','Beans','400','g',TRUE,1),('{soap}','{list_id}','Dish soap','1','bottle',TRUE,2),('{unchecked}','{list_id}','Parsley','5','g',FALSE,3);
INSERT INTO users(id,username,display_name,email) VALUES('{other}','other_{suffix}','Other {suffix}','other-{suffix}@example.test');
INSERT INTO kitchen_profiles(id,owner_id,name,request_id,request_hash) VALUES('{foreign}','{other}','Private','{uuid.uuid4()}','{'c'*64}');
""")
        def record(request):
            if urlsplit(request.url).path.startswith('/api/transfer_purchased_groceries'):paths['transfer']=urlsplit(request.url).path
        page.on('request',record)
        await page.goto(base+f'/kitchen/grocery-lists/{list_id}',wait_until='networkidle')
        await page.get_by_role('button',name='Add purchased to pantry',exact=True).click()
        dialog=page.get_by_role('dialog')
        await expect(dialog.get_by_role('checkbox')).to_have_count(3)
        await dialog.get_by_role('checkbox',name=re.compile('Dish soap')).uncheck()
        for width in [390,1440]:
            await page.set_viewport_size({'width':width,'height':1000})
            await expect(dialog).to_be_visible()
            assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
            await page.screenshot(path=str(args.output/f'put-away-{width}.png'))
        # A request that never reaches the server leaves stock and groceries intact.
        await page.route('**/api/transfer_purchased_groceries*',lambda route:route.abort(),times=1)
        await dialog.get_by_role('button',name='Add to pantry',exact=True).click()
        await expect(dialog).to_contain_text('We couldn’t confirm the save.')
        assert sql(f"SELECT available::text FROM pantry_ingredients WHERE profile_id='{pantry}' AND name='RICE';")=='false'
        await dialog.get_by_role('button',name='Close',exact=True).click()
        await page.get_by_role('button',name='Add purchased to pantry',exact=True).click()
        dialog=page.get_by_role('dialog')
        await dialog.get_by_role('checkbox',name=re.compile('Dish soap')).uncheck()
        async def lose_response(route):await route.fetch();await route.abort()
        await page.route('**/api/transfer_purchased_groceries*',lose_response,times=1)
        await dialog.get_by_role('button',name='Add to pantry',exact=True).click()
        await expect(dialog.locator('.grocery-error')).to_be_visible()
        await expect(dialog).to_contain_text('Refresh your grocery list and pantry before retrying.')
        await expect(dialog.get_by_role('button',name='Add to pantry',exact=True)).to_be_disabled()
        await dialog.get_by_role('button',name='Close',exact=True).click()
        await page.get_by_role('button',name='Add purchased to pantry',exact=True).click()
        dialog=page.get_by_role('dialog')
        await dialog.get_by_role('checkbox',name=re.compile('Dish soap')).uncheck()
        await dialog.get_by_role('button',name='Add to pantry',exact=True).click()
        await expect(dialog).to_contain_text('Your purchased ingredients are available')
        assert sql(f"SELECT count(*) FROM pantry_ingredients WHERE profile_id='{pantry}';")=='2'
        assert sql(f"SELECT amount||'|'||unit||'|'||available::text FROM pantry_ingredients WHERE profile_id='{pantry}' AND name='RICE';")=='2|kg|true'
        assert sql(f"SELECT amount||'|'||unit FROM pantry_ingredients WHERE profile_id='{pantry}' AND name='Beans';")=='400|g'
        assert sql(f"SELECT count(*) FROM grocery_list_items WHERE list_id='{list_id}' AND checked;")=='3'
        await dialog.get_by_role('link',name='Open pantry',exact=True).click()
        await expect(page.get_by_role('heading',name='Home pantry',exact=True)).to_be_visible()
        await page.reload(wait_until='networkidle')
        await expect(page.locator('body')).to_contain_text('Beans')
        payload={'expected_account_id':owner,'list_id':list_id,'expected_revision':0,'pantry_id':foreign,'item_ids':[rice]}
        async def post(data):return await ctx.request.post(base+paths['transfer'],data=json.dumps(data),headers={'Content-Type':'application/json','Origin':base})
        assert (await post(payload)).status>=400
        payload['pantry_id']=pantry;payload['item_ids']=[unchecked]
        assert (await post(payload)).status>=400
        payload['item_ids']=[rice];payload['expected_revision']=99
        assert (await post(payload)).status>=400
        # Removal is opt-in and atomic; a lost response asks the user to refresh.
        await page.goto(base+f'/kitchen/grocery-lists/{list_id}',wait_until='networkidle')
        await page.get_by_role('button',name='Add purchased to pantry',exact=True).click()
        dialog=page.get_by_role('dialog')
        await expect(dialog.get_by_text('Select ingredients to add to a pantry.',exact=True)).to_be_visible()
        await expect(dialog.get_by_role('radio',name='Keep on grocery list',exact=True)).to_be_checked()
        await dialog.get_by_role('checkbox',name=re.compile('Dish soap')).uncheck()
        await dialog.get_by_role('radio',name='Remove from grocery list',exact=True).check()
        await page.set_viewport_size({'width':390,'height':1000})
        await page.screenshot(path=str(args.output/'put-away-remove-390.png'))
        await page.route('**/api/transfer_purchased_groceries*',lose_response,times=1)
        await dialog.get_by_role('button',name='Add to pantry',exact=True).click()
        await expect(dialog.locator('.grocery-error')).to_be_visible()
        await expect(dialog).to_contain_text('Refresh your grocery list and pantry before retrying.')
        await expect(dialog.get_by_role('button',name='Add to pantry',exact=True)).to_be_disabled()
        await dialog.get_by_role('button',name='Close',exact=True).click()
        await expect(page.locator('.grocery-row')).to_have_count(2)
        assert sql(f"SELECT count(*) FROM pantry_ingredients WHERE profile_id='{pantry}';")=='2'
        assert sql(f"SELECT string_agg(name,',' ORDER BY position) FROM grocery_list_items WHERE list_id='{list_id}';")=='Dish soap,Parsley'
        assert not errors,errors
        (args.output/'result.json').write_text(json.dumps({'passed':['purchased-only selection','manual item exclusion','mobile and desktop layout','failed request keeps data and can be retried after refresh','uncertain response requests refresh','case-insensitive existing ingredient keeps amount','new ingredient copies amount','shopping list retained','persisted pantry navigation','foreign pantry rejected','unpurchased item rejected','stale list rejected','optional removal stays atomic after lost response','list refreshed after removal'],'console_errors':errors},indent=2))
        await browser.close()
        print('PASS: purchased grocery put-away flow, persistence, retry and access checks',flush=True)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--instance',default='noms-groceries');parser.add_argument('--output',type=Path,default=Path('/tmp/purchased-e2e'))
    asyncio.run(main(parser.parse_args()))
