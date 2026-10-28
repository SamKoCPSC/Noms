#!/usr/bin/env python3
"""End-to-end pantry checks against a ready local mock-auth Noms stack only.
Requires Python Playwright and Chromium. Cookies stay in memory.
"""
import argparse,asyncio,json,re,uuid
from pathlib import Path
from urllib.parse import urlsplit

async def main(args):
 from playwright.async_api import async_playwright,expect
 # Fresh browser contexts can finish network loading before WASM hydration.
 expect.set_options(timeout=20000)
 assert re.fullmatch(r'noms-[a-z0-9][a-z0-9-]*',args.instance)
 metadata=dict(line.split('=',1) for line in (Path('/tmp/noms-dev')/args.instance/'metadata').read_text().splitlines() if '=' in line)
 base=metadata['APP_URL'];assert metadata['STATE']=='ready' and urlsplit(base).hostname in {'127.0.0.1','localhost','::1'}
 args.output.mkdir(parents=True,exist_ok=True)
 checks=[];errors=[]
 async with async_playwright() as p:
  browser=await p.chromium.launch(executable_path='/usr/bin/chromium',args=['--disable-dev-shm-usage'])
  async def login(context):
   page=await context.new_page();page.set_default_navigation_timeout(120000)
   suffix=uuid.uuid4().hex[:12]
   await page.goto(base+'/login',wait_until='networkidle');await page.get_by_role('link',name='Continue with Google').click()
   assert await page.locator('input[name=username]').count(),'Expected local mock OAuth form'
   await page.locator('input[name=username]').fill('pantry-'+suffix)
   await page.locator('textarea[name=claims]').fill(json.dumps({'email':f'pantry-{suffix}@example.test','email_verified':True,'name':'Pantry Demo'}))
   await page.locator('input[type=submit]').click();await page.wait_for_url(base+'/**')
   owner=(await (await context.request.get(base+'/api/user_profile')).json())['current_user_id']
   return page,owner
  context=await browser.new_context(viewport={'width':1440,'height':1100})
  page,owner=await login(context);page.on('pageerror',lambda e:errors.append(str(e)))
  api_paths={}
  def record_route(request):
   path=urlsplit(request.url).path
   for name in ['get_kitchen','add_pantry_entry']:
    if path.startswith('/api/'+name):
     api_paths[name]=path
  page.on('request',record_route)
  await page.goto(base+'/kitchen/pantry',wait_until='networkidle');await expect(page.locator('h1')).to_have_text('Organize your virtual pantry')
  await page.goto(base+'/kitchen/pantry/new',wait_until='networkidle')
  await page.get_by_role('button',name='Create pantry',exact=True).click();await expect(page.locator('[role=alert]').first).to_contain_text('Pantry name')
  await page.get_by_label('Pantry name',exact=True).fill('Home pantry')
  await page.get_by_label('Note',exact=False).fill('Ingredients for everyday meals.')
  await expect(page.get_by_label('Equipment name',exact=True)).to_have_count(0)
  await expect(page.locator('.pantry-initial-equipment')).to_have_count(0)
  form=page.locator('form.pantry-add').first
  for name,amount,unit in [('Rice','2','cup'),('Olive oil','',''),('Salt','0','g')]:
   await form.get_by_label('Ingredient name',exact=True).fill(name)
   await form.get_by_label('Amount',exact=True).fill(amount);await form.get_by_label('Unit',exact=True).fill(unit)
   await form.get_by_role('button',name='Add ingredient',exact=True).click()
  await page.get_by_role('button',name='Edit initial Rice',exact=True).click()
  draft_dialog=page.get_by_role('dialog')
  await draft_dialog.get_by_label('Ingredient name',exact=True).click()
  await draft_dialog.get_by_role('button',name='Save item',exact=True).click()
  await expect(draft_dialog).to_have_count(0)
  # An entry typed without pressing Add must be retained by Create.
  await form.get_by_label('Ingredient name',exact=True).fill('Lentils')
  await page.screenshot(path=str(args.output/'create-desktop.png'),full_page=True)
  await page.set_viewport_size({'width':390,'height':844});assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth');await page.screenshot(path=str(args.output/'create-phone.png'),full_page=True)
  await page.set_viewport_size({'width':1440,'height':1100})
  await page.get_by_role('button',name='Create pantry',exact=True).click();await page.wait_for_url(re.compile(r'.*/kitchen/pantry/[0-9a-f-]{36}$'))
  home=page.url;home_id=home.rsplit('/',1)[1]
  await expect(page.locator('h1')).to_have_text('Home pantry');await expect(page.locator('.pantry-row')).to_have_count(4)
  await expect(page.locator('.grocery-pill')).to_have_text('3 in stock')
  checks+=['empty state and form validation','mixed-unit/unknown/zero starting inventory','unsubmitted ingredient retained at creation']
  rice=page.locator('.pantry-row').filter(has_text='Rice')
  await page.route('**/api/update_pantry_entry*',lambda route:route.abort())
  await rice.get_by_role('button',name='Mark Rice out',exact=True).click();await expect(page.locator('[role=alert]').first).to_be_visible()
  await expect(rice.get_by_role('button',name='Mark Rice out',exact=True)).to_be_enabled()
  await page.unroute('**/api/update_pantry_entry*')
  await rice.get_by_role('button',name='Mark Rice out',exact=True).click();await expect(rice.get_by_role('button',name='Restore Rice',exact=True)).to_be_visible()
  await page.reload(wait_until='networkidle');await expect(rice).to_contain_text('2 cup');await expect(rice.get_by_role('button',name='Restore Rice',exact=True)).to_be_visible()
  checks+=['failed availability write leaves saved state intact','out status persists without erasing quantity']
  await rice.get_by_role('button',name='Edit Rice',exact=True).click()
  editor=page.locator('.pantry-editor')
  await editor.get_by_label('Ingredient name',exact=True).fill('Brown rice')
  await editor.get_by_label('Amount',exact=False).fill('-1');await editor.get_by_role('button',name='Save changes',exact=True).click()
  await expect(editor.locator('[role=alert]')).to_contain_text('nonnegative')
  await editor.get_by_label('Amount',exact=False).fill('500');await editor.get_by_label('Unit',exact=False).fill('g')
  await editor.get_by_role('button',name='Save changes',exact=True).click();await expect(page.locator('.pantry-row').filter(has_text='Brown rice')).to_contain_text('500 g')
  await expect(page.get_by_role('button',name='Restore Brown rice',exact=True)).to_be_visible()
  checks+=['full edits and negative quantity rejection']
  add=page.locator('form.pantry-add')
  await add.get_by_label('Ingredient name',exact=True).fill('Flour');await add.get_by_label('Amount',exact=True).fill('1 1/2');await add.get_by_label('Unit',exact=True).fill('kg')
  await page.route('**/api/add_pantry_entry*',lambda route:route.abort())
  await add.get_by_role('button',name='Add ingredient',exact=True).click();await expect(page.locator('[role=alert]').first).to_be_visible();await expect(add.get_by_label('Ingredient name',exact=True)).to_have_value('Flour')
  await page.unroute('**/api/add_pantry_entry*');await add.get_by_role('button',name='Add ingredient',exact=True).click();await expect(page.locator('.pantry-row').filter(has_text='Flour')).to_have_count(1)
  # Seed a legacy equipment row through the compatible API, then confirm the
  # ingredient-only UI hides it while preserving the stored record.
  legacy=await context.request.post(base+api_paths['add_pantry_entry'],data=json.dumps({'expected_account_id':owner,'profile_id':home_id,'kind':'Equipment','request_id':str(uuid.uuid4()),'stock':{'fields':{'name':'Legacy blender','amount':'','unit':''},'available':True}}),headers={'Content-Type':'application/json','Origin':base})
  assert legacy.status==200,('Legacy fixture failed',legacy.status)
  await page.reload(wait_until='networkidle')
  await expect(page.locator('.pantry-row')).to_have_count(5)
  await expect(page.get_by_role('group',name='Inventory type',exact=True)).to_have_count(0)
  await expect(page.get_by_label('Equipment name',exact=True)).to_have_count(0)
  await expect(page.locator('.pantry-profile-link').filter(has_text='Home pantry')).to_contain_text('5 ingredients')
  await page.get_by_label('Search inventory',exact=True).fill('Legacy blender');await expect(page.locator('.pantry-row')).to_have_count(0)
  await page.get_by_label('Search inventory',exact=True).fill('')
  saved=await context.request.post(base+api_paths['get_kitchen'],data=json.dumps({'expected_account_id':owner,'id':home_id}),headers={'Content-Type':'application/json'})
  assert saved.status==200
  assert any(entry['kind']=='Equipment' and entry['stock']['fields']['name']=='Legacy blender' for entry in (await saved.json())['entries'])
  checks+=['equipment UI excluded while legacy equipment data is preserved']
  # A separate authenticated browser context sees committed rows.
  second=await browser.new_context(storage_state=await context.storage_state());other_page=await second.new_page();other_page.on('pageerror',lambda e:errors.append(str(e)));await other_page.goto(home,wait_until='networkidle');await expect(other_page.locator('.pantry-row').filter(has_text='Flour')).to_be_visible();await second.close()
  checks+=['manual additions and retained retry fields','database persistence in a second browser session']
  await page.get_by_label('Search inventory',exact=True).fill('Flour');await expect(page.locator('.pantry-row')).to_have_count(1);await page.get_by_label('Search inventory',exact=True).fill('')
  await page.get_by_label('Availability filter',exact=True).select_option('out');await expect(page.locator('.pantry-row')).to_have_count(2);await page.get_by_label('Availability filter',exact=True).select_option('all')
  await page.get_by_role('button',name='Edit Brown rice',exact=True).click()
  for width in [320,390,760,1100,1440]:
   await page.set_viewport_size({'width':width,'height':950});assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth'),f'Pantry overflow {width}'
  await page.set_viewport_size({'width':1440,'height':1100});await expect(page.locator('h1')).to_have_css('font-size','36px');await page.screenshot(path=str(args.output/'workspace-desktop.png'),full_page=True)
  await page.set_viewport_size({'width':390,'height':844});await page.screenshot(path=str(args.output/'workspace-phone.png'),full_page=True)
  await page.set_viewport_size({'width':1440,'height':1100});await page.get_by_role('button',name='Toggle theme',exact=True).click();await expect(page.locator('html')).to_have_class(re.compile('dark'));await page.screenshot(path=str(args.output/'workspace-dark.png'),full_page=True)
  checks+=['search and availability filters','320/390/760/1440px layouts and light/dark themes']
  await page.get_by_role('button',name='Duplicate pantry',exact=True).click();await page.wait_for_url(re.compile(r'.*/kitchen/pantry/(?!'+home_id+r')[0-9a-f-]{36}$'))
  copied=page.url;await expect(page.locator('h1')).to_have_text('Home pantry copy');await expect(page.get_by_role('button',name='Restore Brown rice',exact=True)).to_be_visible()
  await page.get_by_role('button',name='Restore Brown rice',exact=True).click();await expect(page.get_by_role('button',name='Mark Brown rice out',exact=True)).to_be_visible()
  await page.locator('.pantry-profile-link').filter(has_text='Home pantry').filter(has_not_text='copy').click();await expect(page.locator('h1')).to_have_text('Home pantry');await expect(page.get_by_role('button',name='Restore Brown rice',exact=True)).to_be_visible()
  await page.go_back();await expect(page.locator('h1')).to_have_text('Home pantry copy')
  checks+=['independent duplication including out entries','profile switching and browser back']
  await page.get_by_role('button',name='Edit pantry',exact=True).click();dialog=page.get_by_role('dialog');await dialog.get_by_label('Pantry name',exact=True).fill('Cabin');await dialog.get_by_label('Note',exact=True).fill('Weekend cooking');await dialog.get_by_role('button',name='Save pantry',exact=True).click();await expect(page.locator('h1')).to_have_text('Cabin')
  await page.goto(base+'/kitchen/pantry',wait_until='networkidle');await expect(page.locator('h1')).to_have_text('Cabin')
  # Confirm the direct function route and encoding before testing owner rejection.
  get_path=api_paths['get_kitchen']
  valid=await context.request.post(base+get_path,data=json.dumps({'expected_account_id':owner,'id':home_id}),headers={'Content-Type':'application/json'})
  assert valid.status==200,('Owner read API failed',valid.status)
  assert (await valid.json())['id']==home_id
  # Owner isolation through a second real signed-in account and a direct server-function call.
  stranger=await browser.new_context();stranger_page,stranger_owner=await login(stranger)
  response=await stranger.request.post(base+get_path,data=json.dumps({'expected_account_id':stranger_owner,'id':home_id}),headers={'Content-Type':'application/json'})
  assert response.status>=400,('Cross-owner API unexpectedly succeeded',response.status)
  await stranger_page.goto(home,wait_until='networkidle');await expect(stranger_page.locator('.pantry-row')).to_have_count(0);await stranger.close()
  # A spoofed expected account is a consistency violation, never an owner selector.
  response=await context.request.post(base+get_path,data=json.dumps({'expected_account_id':stranger_owner,'id':home_id}),headers={'Content-Type':'application/json'})
  assert response.status>=400
  checks+=['profile metadata editing and remembered active pantry','cross-owner API/UI isolation and account binding']
  await page.get_by_role('button',name='Edit Flour',exact=True).click();await editor.get_by_role('button',name='Remove ingredient',exact=True).click();await page.get_by_role('dialog').get_by_role('button',name='Remove item',exact=True).click();await expect(page.locator('.pantry-row').filter(has_text='Flour')).to_have_count(0)
  await page.get_by_role('button',name='Delete pantry',exact=True).click();await page.get_by_role('dialog').get_by_role('button',name='Delete pantry',exact=True).click();await expect(page.locator('h1')).to_have_text('Home pantry')
  await page.goto(base+'/kitchen/pantry/new',wait_until='networkidle');await page.get_by_label('Pantry name',exact=True).fill('Empty pantry');await page.get_by_role('button',name='Create pantry',exact=True).click();await page.wait_for_url(re.compile(r'.*/kitchen/pantry/[0-9a-f-]{36}$'));await expect(page.locator('.pantry-row')).to_have_count(0)
  await page.get_by_role('button',name='Delete pantry',exact=True).click();await page.get_by_role('dialog').get_by_role('button',name='Delete pantry',exact=True).click();await expect(page.locator('h1')).to_have_text('Home pantry')
  checks+=['ingredient removal and delete-active-profile','empty pantry creation and deletion']
  anonymous=await browser.new_context();anon=await anonymous.new_page();await anon.goto(home,wait_until='networkidle');await expect(anon.locator('.pantry-row')).to_have_count(0);await anonymous.close();checks+=['anonymous inventory isolation']
  assert not errors,errors
  result={'passed':True,'browser':browser.version,'checks':checks,'page_errors':errors};(args.output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));await browser.close()
if __name__=='__main__':
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--instance',default='noms-groceries');parser.add_argument('--output',type=Path,default=Path('/workspace/noms-pantry-e2e'));asyncio.run(main(parser.parse_args()))
