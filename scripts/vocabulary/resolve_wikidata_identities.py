#!/usr/bin/env python3
"""Resolve the first Wikidata identity queue without merging published UUIDs.

Names propose associations only; recorded food scope and part/form checks control
admission. Existing duplicate identities are retained, with one representative
receiving related evidence. Every queued entity gets an auditable case decision.
"""
import argparse
import collections
import copy
import json
from pathlib import Path
import re
import unicodedata

from foodon import ROOT, load, normalize, require, write_bundle
from review_wikidata import disposition
import wikidata as wd

PREFIX = 'Identity review v1: '
SINGULAR = {'cheeses':'cheese','leaves':'leaf','berries':'berry','tomatoes':'tomato','potatoes':'potato',
            'eggs':'egg','seeds':'seed','nuts':'nut','beans':'bean','peppers':'pepper','mushrooms':'mushroom',
            'yogurt':'yoghurt','yoghurts':'yoghurt','onions':'onion','carrots':'carrot','grapes':'grape',
            'dried':'dry','tinned':'canned','sprouts':'sprout','cloves':'clove','cows':'cow','goats':'goat',
            'filet':'fillet','filets':'fillet','fillets':'fillet','trotters':'trotter'}
GENERIC = {'cheese','milk','food','ingredient','fruit','vegetable','meat','fish','sauce','spice','rice','oil',
           'seed','flour','butter','bread','egg','cream','salt','sugar','paste','condiment','animal','plant',
           'cereal','fillet','starch','colostrum','bushmeat','fruit vegetable','non-nutritive sweetener',
           'gizzard','loin','knuckle','shank','saddle','rump','haunch','breast','tongue','head','ear',
           'brain','marrow','offal','feet','udder','neck','ribs'}


def signature(text, family=None):
    text = ''.join(c for c in unicodedata.normalize('NFKD', text).lower() if not unicodedata.combining(c))
    text = re.sub(r'\bas food\b|\b(?:pdo|pgi|pat)\b|\(food\)', ' ', text)
    text = re.sub(r"[’']s\b", '', text)
    text = re.sub(r"[’'`ʿʻʼ]", '', text)
    words = [SINGULAR.get(w,w) for w in re.findall(r'[a-z0-9]+',text)]
    if family=='meat':
        words=[w for w in words if w!='of']
        words=[{'bovine':'beef','cattle':'beef','porcine':'pork','pig':'pork','caprine':'goat'}.get(w,w) for w in words]
        if 'meat' in words and len(words)>1:words.remove('meat')
    # Suffixes are cosmetic only inside a recorded food family. Other form words survive.
    suffix = {'cheese':'cheese','condiment':'sauce'}.get(family)
    if suffix in words and len(words)>1:
        words.remove(suffix)
    return ' '.join(sorted(words))


def food_family(label, description):
    patterns=[
        ('cheese',r'\bcheese\b'),('condiment',r'\b(?:sauce|seasoning|spice|condiment|chutney|sambal|masala|ketchup|mustard)\b'),
        ('dairy',r'\b(?:milk|butter|yoghurt|yogurt|cream|curd|whey)\b'),
        ('meat',r'\b(?:meat|sausage|offal|salted fish|smoked fish|cut of|cut from|fillet|steak|ham|bacon|beef|pork|veal|lamb|mutton|chicken|poultry|turkey)\b'),
        ('sweet',r'\b(?:honey|syrup|sugar|jam|jelly|chocolate|cocoa|icing|frosting|fondant|molasses|marzipan|preserve)\b'),
        ('grain',r'\b(?:flour|bread|dough|rice|grain|starch|bran|kernel|groat|malt|cereal|barley|wheat|oat|rye)\b'),
        ('oil',r'\boil\b'),('produce',r'\b(?:fruit|vegetable|edible|nut|seed|leaf|leaves|root|tuber|bulb|berry|flesh)\b'),
        ('pantry',r'\b(?:vinegar|stock|broth|gelatin|gelatine|yeast|starter|juice|puree|purée|paste|spread|pickled|pickle|fermented)\b')]
    for text in [label,label+' '+description]:
        for family,pattern in patterns:
            if re.search(pattern,text,re.I):return family
    return None


def scope_hold(term):
    """These checks precede fuzzy/alias matching, including apparently familiar words."""
    label, description=term['label'],term['description']
    text=label+' '+description
    if re.search(r'\b(?:food class|ontology term|grouping of product types|class for .+ FMMO|economic activity|sale of a food animal|applying salt or sugar)\b',description,re.I) or re.search(r'\b(?:food product|food material|fish product|soft cow dairy product|milk coagulants|milk lactone|Milk of advanced lactation|non-avian gizzard|mystery meat|Government cheese)\b',label,re.I):
        return 'classification_or_property','Definition describes an umbrella, technical/biological material, activity or distribution condition: '+description
    if re.search(r'\b(?:medicinally|medically|medication|medicine|therapeutic|remedy|plague|Ayurvedic|illness|laxative|purgative|pancreatic|amylases?|digestive juice|gastric|saliva|glycosides?|oxidase|esterase)\b',text,re.I):
        return 'medical_or_biochemical_material','Pinned text identifies a medical/biochemical use or material rather than an initial cooking ingredient: '+description
    if re.search(r'\b(?:confectionery|pralines?|chocolate (?:coins?|frogs?|eggs?|buttons?)|candy bars?|wafer|Cadbury|Sonho de Valsa|DARS|Caramello|LOOK|Camp Coffee|Fox.s U-bet|Chocolate Panda)\b',text,re.I):
        return 'finished_confection_or_brand','Finished confection or named commercial sweet; generic baking materials remain separate.'
    if re.search(r'\b(?:standard|classification|regulation|sugar-free product|flavoring|flavouring|flavoured syrup|flavored syrup|organic chocolate|raw chocolate|craft chocolate|military chocolate|cyclic salt|government.s stockpiled rice)\b',text,re.I):
        return 'classification_or_property','Source names a standard, role, certification, distribution condition or broad property, not a distinct ingredient.'
    if re.match(r'^(?:(?:a|an|the) )?(?:[\w-]+ ){0,2}(?:hamlet|commune|village|settlement|town|city|mythological creature|ritual|ceremony|practice|method)\b',description,re.I):
        return 'nonfood_subject','The definition describes a place, practice or fictional subject, not an edible material: '+description
    checks=[
        ('nonfood_or_brand',r'\b(?:company|brand|trademark|trademarked|trade mark|manufacturer|cheesemaker|store|corporation|restaurant|restaurants|video game|fictional|Papa.s|Paleteria|museum|specimen|collection of|artistic|instrument|container|utensil|straw|tableware|tool|fossil|coin|sauceboat|Kraft|Heinz|Danone|Nestlé|Nestle|industrial cheese)\b'),
        ('biological_or_industrial',r'\b(?:enzyme|enzymes|InterPro|protein family|mammalian protein|biological process|gene|isoform|stereoisomer|chemical compounds?|chemical additive|inorganic|antioxidant|processing aid|acidulant|acidifier|anticaking|humectant|firming agent|gelling agent|glazing agent|preservative|packaging gas|antifoaming|buffer|medication|medicinal|pharmaceutical|supplement|appetite|growth promoter|cosmetic|cannabis|marijuana|resin|essential oil|fungal|molds|moulds)\b'),
        ('non_cooking_material',r'\b(?:humans?|breast ?milk|wet nurse|homo sapiens|purgative|toxic|poisonous|inedible|tung oil|croton oil|castor oil|jojoba oil|eucalyptus oil|cypress oil|monoi oil|mad honey|honey oil|Dead Sea salt|pig louse|cuckoo|ornamental|koi|Cyprinus rubrofuscus)\b'),
        ('prepared_dish_or_method',r'\b(?:dish|dessert|sandwich|casserole|stew|salad|pizza|pudding|dumpling|sushi|risotto|biryani|omelette|taco|burrito|cake|torte|porridge|popsicle|ice lolly|ice pop|meatballs?|meatloaf|schnitzel|cooking technique|cooking method|recipe|Bengali cuisine|percussion)\b'),
        ('taxon_or_plant_identity',r'\b(?:species|genus|cultivar|variety of apple|apple variety|plant variety|strain|breed|nothospecies|hybrid plant|hybrid melon|hybrid squash)\b'),
        ('historical_or_undefined_scope',r'\b(?:undefined|undefinded|unknown|forgotten|historical medication|medieval|18th.century|19th.century|ancient Roman|obscure)\b'),
    ]
    if 'Q431289' in term['instance_of']:
        return 'nonfood_or_brand','Source is explicitly an instance of brand.'
    if 'Q4886' in term['instance_of']:
        return 'taxon_or_plant_identity','Source is explicitly a cultivar, not a separately identified edible material.'
    if re.search(r'\b(?:steroid|glycoside|chemical (?:compound|substance|product)|industrial|ritual|discarded|waste|repeatedly heated|overheated|degraded|contaminated|swizzle stick|oil mist|aerosol|contrast enhancers|dough conditioner|bleaching agent)\b',text,re.I):
        # Compound butter/chocolate are culinary preparations, not chemical compounds.
        if not re.search(r'\bcompound (?:butter|chocolate|sauce)\b',text,re.I):
            return 'technical_or_noningredient_material','Source describes a technical material, processing role, artifact or degraded product: '+description
    if re.search(r'\b(?:cheese classification|cheeses produced|types of cheese|type of cheese by)\b',description,re.I) or re.search(r'^(?:Cheese in .+|(?:French|British|Danish|Georgian|Italian|Tibetan|Russian|Artisan|artisan|dairy|semi-solid semi-hard|raw-milk|semi-hard|mixed milk|white mold-rind|washed-rind|pressed (?:uncooked|cooked|half-cooked)|low-fat|semi-fat|extra-fat|medium-moisture) cheese)$',label,re.I):
        return 'organizational_cheese_category','Geographic, manufacturing, fat/moisture or rind classification, not a named cheese identity.'
    if re.search(r'^(?:cut of meat|white meat|fresh meat|fatty meat|lean meat|poultry meat|marine mammals as food|insect as food|bivalve as food|unusually shaped vegetable|perennial vegetable|Asian vegetable|stem vegetables|starchy root vegetable|winter grain|whole grain|ice cream flavor|vegetable dip|instant sauce|special sauce|cream and white sauce|marinades & grilling sauce|bread and pastry dough|cookie and brownie dough|seed shell oil|oil from the endosperm)$',label,re.I):
        return 'generic_or_functional_group','Source groups multiple foods or describes an incidental property rather than a distinct ingredient.'
    if re.search(r'\b(?:ice[ -]cream|gelato|flavou?r sold|sugared ice|snow cone|shaved ice drink)\b',text,re.I):
        return 'complete_frozen_dessert','Named finished dessert/flavour variant rather than an initial pantry ingredient.'
    if re.search(r'\b(?:Edy.s Pie|Yakult|Vegeta|Vegemite|HP Sauce|Kitchen Bouquet|Old Bay Seasoning|Texas Pete|Secret Stadium|Maggi seasoning|McDonald|Whamco|W.ham|Bárbaro Mojo|Ben & Jerry|Viandox|Henderson.s Relish|Ah-So|Chef Brown Sauce|Blair.s Death Sauce|Drumstick|Cornetto|Yop|Jif)\b',text,re.I):
        return 'commercial_product_scope','Named commercial product is held for product-versus-generic identity review; do not infer a generic equivalent.'
    for reason,pattern in checks:
        if re.search(pattern,text,re.I):return reason,'Pinned label/description: '+description
    if re.search(r'\b(?:iformes|aceae|idae)\b',label) or re.search(r'(?:iformes|aceae|idae) as food$',label):
        return 'taxonomic_food_group','Display label names a biological order/family rather than a specific culinary food.'
    if normalize(label) in GENERIC or re.search(r'\b(?:substitute|product category|class of|type of thickener|type of food|processing|technology|quality condition|carcass quality|treatment agent|flour improver|raising agent)\b',text,re.I):
        return 'generic_or_functional_group','Source describes an umbrella, role or condition: '+description
    if re.search(r'^(?:head|brain|intestines|bone|bone marrow|blood|leg|belly|wing|egg|penis|ear|snout|skin|heart|tail|fat|trotter|cheek|eye|feet|gizzard|stomach|udder|neck|testicle|spleen|tendon)s? as food$',label,re.I):
        return 'unspecified_animal_part','Edible part lacks animal specificity; do not equate different species or prepared products.'
    return None


def ingredient_evidence(term, family):
    label, description=term['label'],term['description']
    text=label+' '+description
    if re.search(r'\b(?:stock|broth)\b',label,re.I) and normalize(label) not in {
        'shrimp stock','fish broth','chicken broth','beef broth','vegetable broth','lamb stock',
        'goat meat stock','game stock','game broth','goose broth','chickpea broth','pea stock'}:
        return False,'Specific recipe/organ/multi-ingredient broth variant is held at the initial pantry granularity: '+description
    if term.get('identity_type_evidence') and family not in {'oil'}:
        return True,'Pinned non-deprecated food-type path: '+term['identity_type_evidence']+'. Named edible type; food form and scope retained.'
    if re.search(r'\b(?:protected designation|protected geographical|PDO|PGI|PAT)\b',text) and not re.search(r'\b(?:cheese|olive oil|honey|ham|sausage|fruit|vegetable|rice|flour|meat|salt)\b',description,re.I):
        return False,'Geographical designation alone does not specify the food identity.'
    if family=='cheese' and re.search(r'\bcheese\b',description,re.I):
        return True,'Named cheese; pinned description identifies the cheese food material.'
    if family=='condiment' and re.search(r'\b(?:sauce|seasoning|spice|condiment|chutney|sambal|masala)\b',description,re.I):
        return True,'Named seasoning/condiment; source description establishes culinary use.'
    if family=='meat' and re.search(r'\b(?:flesh|meat|sausage|offal|cut of|cut from|fillet|steak|ham|bacon|cured|salted|smoked)\b',description,re.I):
        return True,'Specific edible animal material, cut or preserved product; form is retained.'
    if family=='dairy' and re.search(r'\b(?:milk|butter|yogurt|yoghurt|cream|curd|whey|dairy)\b',description,re.I):
        return True,'Named dairy ingredient/preparation; source defines the food form.'
    if family=='sweet' and re.search(r'\b(?:honey|syrup|sugar|jam|jelly|chocolate|cocoa|icing|frosting|fondant|molasses|marzipan|preserve)\b',description,re.I):
        return True,'Named sweetener, preserve or baking ingredient; source describes the edible product.'
    if family=='grain' and re.search(r'\b(?:flour|bread|dough|rice|grain|starch|bran|kernel|groat|malt)\b',description,re.I):
        return True,'Named grain or baking ingredient; processing form is retained.'
    if family=='produce' and re.search(r'\b(?:fruit|vegetable|edible|nut|seed|leaf|leaves|tuber|root|bulb|berry|flesh)\b',description,re.I):
        return True,'Explicit edible plant/animal material in the pinned description; named part is retained.'
    if family=='pantry' and re.search(r'\b(?:vinegar|stock|broth|gelatin|gelatine|yeast|starter|juice|puree|purée|paste|spread|pickled|pickle|fermented)\b',description,re.I):
        return True,'Named pantry product; the source describes a specific food preparation.'
    if family=='oil' and re.search(r'\b(?:edible|cooking|vegetable oil|olive oil)\b',description,re.I):
        if not re.search(r'\b(?:olive|apricot|babassu|sesame|turnip|rapeseed|soybean|canola|cooking spray|diacylglycerol)\b',text,re.I):
            return False,'Vegetable/seed-oil origin alone does not establish ordinary culinary use: '+description
        return True,'Source explicitly identifies an edible/culinary oil.'
    return False,'Pinned text does not establish a sufficiently specific culinary identity: '+description


def typed_family(term, terms):
    """Use retained best-ranked P31/P279 paths to explicit edible-material classes."""
    classes={
        'cheese':'cheese','sauce':'condiment','condiment':'condiment','spice':'condiment','spice mix':'condiment',
        'dairy product':'dairy','milk':'dairy','butter':'dairy','cream':'dairy','yogurt':'dairy',
        'meat':'meat','sausage':'meat','offal':'meat','fish as food':'meat','seafood':'meat',
        'flour':'grain','bread':'grain','rice':'grain','cereal grain':'grain',
        'honey':'sweet','syrup':'sweet','jam':'sweet','chocolate':'sweet','sugar':'sweet',
        'vegetable':'produce','fruit':'produce','onion':'produce','tomato':'produce','potato':'produce',
        'garlic':'produce','edible fruit':'produce','leaf vegetable':'produce',
        'vegetable oil':'oil','olive oil':'oil','cooking oil':'oil',
        'vinegar':'pantry','stock':'pantry','broth':'pantry','juice':'pantry','food paste':'pantry','table salt':'pantry',
    }
    queue=[(q,[term['id'],q]) for q in term['instance_of']+term['subclass_of']]
    seen={term['id']}
    while queue:
        q,path=queue.pop(0)
        if q in seen or q not in terms:continue
        seen.add(q);parent=terms[q]
        label=normalize(parent['label'])
        if label in classes:return classes[label],' → '.join(path)+' ('+parent['label']+')'
        if len(path)<5:
            queue.extend((p,path+[p]) for p in parent['instance_of']+parent['subclass_of'])
    return None,None


def representative(ids, entries, source_label):
    def score(eid):
        e=entries[eid]
        return (e['provenance']['origin']!='curated', normalize(e['label'])!=normalize(source_label),
                ',' in e['label'], len(e['label']), e['id'])
    return min(ids,key=score)


def match_existing(term, family, entries, index):
    def find(name):
        return set(index.get((None,signature(name)),set())) | set(index.get((family,signature(name,family)),set()))
    primary=find(term['label'])
    if family=='meat':
        for animal in ['beef','pork','veal','lamb','mutton','chicken','poultry','turkey','goat']:
            if re.search(r'\b'+animal+r'\b',term['description'],re.I):primary |= find(animal+' '+term['label'])
    if primary:
        # Prefer an actual matching display label over another ingredient's broad alias.
        direct={eid for eid in primary if signature(entries[eid]['label'],family)==signature(term['label'],family)}
        candidates=direct or primary
        if len(candidates)==1:return candidates,'primary_label_variant'
        signatures={signature(entries[eid]['label'],family) for eid in candidates}
        if len(signatures)==1:return candidates,'existing_duplicate_name_family'
        return candidates,'ambiguous_existing_identity'
    candidates=set()
    for alias in term['aliases']:
        if signature(alias) and normalize(alias) not in GENERIC:
            candidates |= find(alias)
    # Source aliases propose related evidence only, never equivalence or new aliases.
    # A precise multiword alias is usable when the source family's type is absent
    # from an otherwise matching canonical definition; conflicting known types stay held.
    candidates={eid for eid in candidates if food_family(entries[eid]['label'],entries[eid]['description']) in {family,None}}
    if len(candidates)==1:return candidates,'scoped_alias_variant'
    if candidates:return candidates,'ambiguous_alias_scope'
    return set(),None


def resolve(args):
    descriptor, base, sources, terms=wd.context(args)
    policy=load(args.policy)
    initial=[d for d in policy['decisions'] if not d['note'].startswith(PREFIX)]
    original={d['term_id']:d for d in initial}
    registry=load(args.registry)
    prior_ids={d['term_id']:d['canonical_id'] for d in registry['decisions'] if d['action']=='import'}
    entries={e['id']:copy.deepcopy(e) for e in base['entries'] if e['status']=='active'}
    for d in initial:
        if d['action']=='import':
            eid=prior_ids[d['term_id']]
            entries[eid]=dict(id=eid,**d['canonical'],provenance={'origin':'imported'},status='active')
    index=collections.defaultdict(set)
    for eid,e in entries.items():
        family=food_family(e['label'],e['description'])
        for name in [e['label'],*e['aliases']]:
            index[(None,signature(name))].add(eid)
            if family:index[(family,signature(name,family))].add(eid)
    base_index=wd.name_index(base)
    queue=[]
    for qid,term in terms.items():
        if qid in original or disposition(term,policy)!='review_culinary_candidate':continue
        matches=set().union(*(base_index[normalize(n)] for n in [term['label'],*term['aliases']] if n))
        labels=base_index[normalize(term['label'])]
        if not (len(matches)==1 and labels==matches):queue.append(qid)
    overrides=policy.get('identity_overrides',{})
    require(set(overrides)<=set(queue),'Identity override is outside the original unresolved queue')
    new, audit, groups=[],[],{}
    for qid in queue:
        t=terms[qid];family=food_family(t['label'],t['description'])
        typed,evidence=typed_family(t,terms)
        if typed and (family in {None,'produce',typed} or family=='dairy' and typed in {'cheese','condiment'}):
            family=typed;t={**t,'identity_type_evidence':evidence}
        row=dict(term_id=qid,label=t['label'],description=t['description'],family=family)
        if qid in overrides:
            d=copy.deepcopy(overrides[qid]);d['term_id']=qid;d['note']=PREFIX+d['note']
            new.append(d);audit.append({**row,'resolution':'explicit_'+d['action'],'note':d['note']});continue
        hold=scope_hold(t)
        if hold:
            reason,note=hold
            d=dict(term_id=qid,action='hold',note=PREFIX+reason+'. '+note)
            new.append(d);audit.append({**row,'resolution':'hold_'+reason,'note':d['note']});continue
        hits,method=match_existing(t,family,entries,index)
        if hits:
            if method.startswith('ambiguous'):
                d=dict(term_id=qid,action='hold',note=PREFIX+method+'. Candidate canonical IDs: '+', '.join(sorted(hits))+'. Distinct identities and authored text retained; no string merge.')
                resolution='hold_'+method
            else:
                eid=representative(hits,entries,t['label'])
                d=dict(term_id=qid,action='map',canonical_id=eid,relationship='related',
                    note=PREFIX+method+'. Related source evidence for '+entries[eid]['label']+'. Scope: '+t['description']+'. Existing UUIDs, forms and wording retained; no exact-equivalence or merge claim.')
                resolution='map_existing'
            new.append(d);audit.append({**row,'resolution':resolution,'candidate_ids':sorted(hits),'note':d['note']});continue
        admitted,note=ingredient_evidence(t,family)
        if not admitted:
            d=dict(term_id=qid,action='hold',note=PREFIX+'insufficient_culinary_scope. '+note)
            new.append(d);audit.append({**row,'resolution':'hold_insufficient_culinary_scope','note':d['note']});continue
        key=(family,signature(t['label'],family))
        if key in groups:
            target=groups[key]
            d=dict(term_id=qid,action='map',target_term_id=target,relationship='related',
                   note=PREFIX+'source_name_family. '+note+' Related evidence for the reviewed '+terms[target]['label']+' identity; source IDs do not imply exact equivalence.')
            resolution='map_new_name_family'
        else:
            label=re.sub(r'\s+as food$','',t['label'],flags=re.I)
            label=re.sub(r'\s+(?:PDO|PGI|PAT)$','',label).strip()
            if family=='oil' and 'olive oil' in t['description'].lower() and not re.search(r'\b(?:oil|olio|azeite|aceite|huile|ulje)\b',label,re.I):
                label += ' olive oil'
            d=dict(term_id=qid,action='import',relationship='exact',canonical=dict(label=label,aliases=[],description=t['description']),
                   note=PREFIX+family+'. '+note+' No maintained label/alias identity match found after scoped spelling/word-order review; source forms retained and no aliases bulk imported.')
            groups[key]=qid;resolution='import'
        new.append(d);audit.append({**row,'resolution':resolution,'note':d['note']})
    result=copy.deepcopy(policy)
    result['decisions']=initial+new
    result['identity_review_version']='v1'
    summary=dict(reviewed=len(queue),resolutions=dict(sorted(collections.Counter(r['resolution'] for r in audit).items())))
    write_bundle(args.output,{'policy.json':result,'identity-review.json':audit,'summary.json':summary})
    return summary


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--snapshot',type=Path,required=True)
    p.add_argument('--descriptor',type=Path,default=ROOT/'data/vocabulary/adapters/wikidata.json')
    p.add_argument('--seed',type=Path,required=True)
    p.add_argument('--sources',type=Path,required=True)
    p.add_argument('--policy',type=Path,default=ROOT/'data/vocabulary/adapters/wikidata-cooking-policy.json')
    p.add_argument('--registry',type=Path,default=ROOT/'data/vocabulary/adapters/wikidata-cooking-review.json')
    p.add_argument('--output',type=Path,required=True)
    print(json.dumps(resolve(p.parse_args())))


if __name__=='__main__':main()
