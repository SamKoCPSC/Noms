#!/usr/bin/env python3
"""Prepare the cooking-focused FoodOn seed from an explicit selection policy.

This is batch curation under the user's cooking-ingredient scope, not an assertion
of individual human review of every source annotation. Output decisions persist
UUIDv4 identities and are consumed by the existing checksum-verifying adapter.
"""
import argparse
import collections
import copy
import json
from pathlib import Path
import re
import uuid

from foodon import ROOT, digest, load, normalize, require, valid_text, write_bundle
from audit_foodon_labels import CLASSIFICATION, named_descendants

DISH_EXCEPT = '\\b(?:bread|crumb|flour|mix|powder|stock|broth|soup|sauce|dressing|pastry|dough|wrapper|wafer|cracker|biscuit|cookie|ladyfinger|filling|icing|frosting|topping|ice cream|chocolate|meringue|couscous)\\b'

DISH = '\\b(?:pizza|sandwich|dinner|meal|casserole|stew|salad|cake|doughnut|donut|pie|tart|muffin|pudding|dessert|pancake|waffle|fritter|omelet|omelette|hamburger|burger|hash|porridge|risotto|paella|pilaf|biryani|curry|sushi|sashimi|lasagna|lasagne|ravioli|tortellini|dumpling|tamal|tamale|taco|burrito|pupusa|arepa|tiramisu|cobbler|crumble|strudel|brownie|croissant|beignet|pasty|quiche|souffle|soufflé|samosa|spring roll|egg roll|eggroll|egg foo|chow mein|chow-mein|chop suey|chop-suey|meatloaf|meat loaf|falafel|falafal|kebab|kebap|kibbeh|hummus|guacamole|baklava|halva|halvah|halwa|kheer|ras malai|rice puto|panipuri|panch phoron|pasta primavera)\\b'

KEEP_DISH = '\\b(?:curry (?:paste|powder|leaf|leaves)|hummus|guacamole|panch phoron|pie (?:crust|shell)|tart shell|tortilla|taco shell)\\b'

GENERIC = '\\b(?:classification|regulatory|system for|by characteristic|by process|by consumer|by component|by meal|consumer group|multi-component|animal-derived|plant-derived|derived food|food item|food material|food substance|food component|miscellaneous|unspecified|not specified|unidentified|unclassified|other |other$|similar|analogues?|imitates?|substitutes?|alternatives?|generic|assorted|assortment|and/or|and other|or other|meat/poultry|raw material|byproduct|by-product|carcass|non-edible|inedible|unknown|unprocessed food|unprepared food|prepared food|processed food|preserved food|formulated food|formulated mix|dry mix|food mix|beverage base|dry beverage base)\\b'

MED = '\\b(?:supplement|supplements|infant|formula|formulae|therapeutic|enteral|medical|toddler|dietetic|invalids|nutritional powder|multivitamin|tablet|tableted|capsule|capsules|special dietary|special nutritional|sports people|sporting people|probiotic|weight reduction|meal replacement)\\b'

INDUSTRIAL = '\\b(?:artificially|artif?icial|artiticial|retention|extruded|irradiated|fortified|enriched|bleached|microwaved|diluted|reconstituted|processed|sterilized|aseptic|vacuum|coated|genetically|detreated|deboned mechanically|mechanically|chilled|bottled|packed|packaged|unpackaged|packaging|quick-frozen|deep-frozen|ultrafiltrated|ultrafiltered|stabilized|unsweetened flavour|carbonated|vitamin|flavoured|flavored|flavoured|flavoured|flavoring|flavouring|flavourings|flavours|flavors)\\b'

GENERIC_FULL = '^(?:food|foods|food product|plant|animal|meat|meat product|fruit|fruit product|vegetable|vegetable product|grain|cereal|cereal grain|legume|pulse|nut|seed|spice|spices|herb|herbs|fat|oil|milk|fish|shellfish|seafood|poultry|bird|mammalian|mammal|mammalian meat|mammalian milk|mammalian milk beverage|mammalian milk butter|mammalian dairy|cow|cattle|bovine|ovine|caprine|chicken|pig|swine|sheep|goat|horse|donkey|duck|goose|turkey|rabbit|buffalo|yak|zebra|reindeer|deer|elk|antelope|camel|camelid|equine|cervid|water buffalo|mammal based|animal based|plant based|animal seafood|fungus vegetable|fungus|algae|aquatic|vegetable oil or fat|animal fat or oil|sea animal|food rind or crust|casing or rind|food casing|food additive|emulsifier|stabilizer|food stabilizer|flavour enhancer|flavor enhancer|edible offal|dietary fiber|starch food|dairy|fermented dairy|animal dairy|bovine dairy|ovine dairy|caprine dairy|plant beverage|animal beverage|seed beverage|plant seed|vegetable root|vegetable leaf|plant shoot|plant stem|edible plant|food plant|fodder|protein|protein food|starch|starch product|cereal food|bakery|confectionery|snack|beverage|fermented beverage|fermented fruit|processed meat|poultry meat|meat food|animal product|plant product|vegetable food|fermented vegetable|sauce|condiment|seasoning|flavor|flavour|tea beverage|juice|fruit juice|vegetable juice)$'

COMMON_CHEM = {'citric acid', 'tartaric acid', 'lactic acid', 'ascorbic acid', 'sodium bicarbonate', 'sodium hydrogen carbonate', 'ammonium bicarbonate', 'ammonium hydrogen carbonate', 'sodium carbonate', 'calcium carbonate', 'calcium chloride', 'sodium chloride', 'potassium chloride', 'calcium sulphate', 'calcium sulfate', 'potassium hydrogen tartrate', 'monosodium glutamate', 'pectin', 'agar', 'agar-agar', 'carrageenan', 'xanthan gum', 'guar gum', 'locust bean gum', 'gum arabic', 'lecithin', 'soy lecithin', 'sorbitol', 'xylitol', 'erythritol', 'stevia', 'sucralose', 'gelatin', 'gelatine', 'glycerol', 'glycerin', 'glycerine', 'inulin', 'maltodextrin', 'dextrose', 'glucose', 'fructose', 'sucrose', 'lactose', 'maltose'}

TYPO = '\\b(?:acifified|meshanger|ieveque|vichysoise|liquer|strawb?ery|artiticial|artifically|unkown|squaw|magic mushroom)\\b'

KEEP_EXACT = {'FOODON_03301424', 'FOODON_00002746', 'FOODON_00002977', 'FOODON_00004437', 'FOODON_03310321', 'FOODON_00003396'}

DEFER_EXACT = {'FOODON_00003441', 'FOODON_00003752', 'FOODON_00003717'}

def clean_label(label):
    label = re.sub('\\s*\\((?:efsa foodex2|gs1 gpc|us cfr|eurofir|codex|ccpr|ec|efg|eurocode2)\\)$', '', label, flags=re.I)
    label = re.sub('^\\d+\\s+(?:-\\s+)?', '', label)
    label = re.sub('\\s*\\(food product\\)', '', label, flags=re.I)
    label = re.sub('\\bfood product\\b', '', label, flags=re.I)
    label = re.sub('^piece of ', '', label, flags=re.I)
    label = ' '.join(label.split()).strip(' ,-')
    return label

def initial_screen(t, context):
    label = t['label']
    n = normalize(label)
    tid = t['id']
    target = clean_label(label)
    nt = normalize(target)
    if tid in DEFER_EXACT:
        return ('hold_definition', target)
    if tid in KEEP_EXACT:
        return ('include', target)
    if tid in context['feed']:
        return ('animal_feed', target)
    if tid in context['hazard']:
        return ('hazard_category', target)
    if tid in context['manual'] and context['manual'][tid]['recommendation'] in {'exclude_initial', 'defer_complete_dish', 'hold_for_source_check', 'hold_for_editorial_review'}:
        return ('editorial_hold', target)
    if not valid_text(target):
        return ('invalid_label', target)
    if re.search(MED, label, re.I):
        return ('medical_or_supplement', target)
    if re.search('\\b(?:plant|plants|tissue|organism|material|resource|substance)\\b', target, re.I):
        return ('noningredient_material_or_plant', target)
    if re.search(GENERIC, target, re.I) or re.fullmatch(GENERIC_FULL, nt):
        return ('generic_or_ambiguous_group', target)
    if re.search('\\b(?:pet|dog food|cat food|feed|fodder|forage|hay|silage|stubble|stover|waste|medicated|poisonous|spoiled|hazardous|contaminated|burns|burn hazard|temperature hazard)\\b', target, re.I):
        return ('non_cooking', target)
    if re.search('\\b(?:industrial|food additive|additive function|raising agent|gelling agent|colour agent|color agent|anti.caking|sequestrant|emulsifier|preservative|processing aid|propellant|glazing agent)\\b', target, re.I):
        return ('industrial_role', target)
    code = bool(CLASSIFICATION.search(label))
    if code and '(efsa foodex2)' not in label.lower():
        return ('classification_category', target)
    num = int(tid[-8:])
    if 3412972 <= num <= 3413399 and nt not in COMMON_CHEM:
        return ('industrial_additive', target)
    if 3544170 <= num <= 3544522 and nt not in COMMON_CHEM:
        return ('industrial_flavour_or_nutrient', target)
    if 3543440 <= num <= 3543720:
        return ('herbal_infusion_catalog', target)
    if re.search(INDUSTRIAL, target, re.I):
        return ('industrial_or_incidental_variant', target)
    if re.search(TYPO, target, re.I):
        return ('source_text_hold', target)
    if re.search('\\b(?:soup|stew|salad)\\b', target, re.I) and (not re.search('\\b(?:mix|powder|stock|broth|canned|dressing|condiment)\\b', target, re.I)):
        return ('prepared_dish', target)
    if re.search(DISH, target, re.I) and (not re.search(DISH_EXCEPT, target, re.I)) and (not re.search(KEEP_DISH, target, re.I)):
        return ('prepared_dish', target)
    if re.search('\\((?:dish|served|cooked, frozen|prepared)\\)', target, re.I):
        return ('prepared_dish', target)
    if re.search('\\b(?:cooked|boiled|fried|baked|sauteed|sautéed|grilled|steamed)\\b', target, re.I) and (not re.search('\\b(?:rice|bean|chickpea|lentil|sausage|ham|bacon|roasted|canned)\\b', target, re.I)):
        return ('prepared_or_cooked_variant', target)
    return ('include', target)

def display_label(label):
    label = clean_label(label)

    def qualifier(m):
        q = m.group(1)
        if q.lower() in {'food product', 'whole or pieces', 'raw', 'fresh', 'uncooked', 'whole or pieces, raw', 'whole or pieces, fresh'}:
            return ''
        q = re.sub('(?:,\\s*)?\\braw\\b(?:,\\s*)?', lambda x: ', ' if x.group(0).startswith(',') and x.group(0).endswith(', ') else '', q, flags=re.I).strip(' ,')
        return '(' + q + ')' if q else ''
    label = re.sub('\\(([^()]*)\\)', qualifier, label)
    return ' '.join(label.split()).strip(' ,-')

EXTRA_GENERIC = '\\b(?:additional|not listed|elsewhere|n\\.e\\.c|analogue?|analog|analogues|primary derivatives|group|grouping|family|class|classes|category|categories|agency type|sugars|polysaccharides|nutrients|fibres|sweeteners|formulation|formulations|source|sources|unconventional|digestible|indigestible|animal based|plant based|animal and vegetable|animal-derived|plant-derived|partially-refined|refined food|cultivated edible|edible fruit|edible seed|vegetable cluster|animal muscle|animal organ|animal tissue|product obtained|fats and oils|human milk|breast milk|breastmilk)\\b'

EXTRA_MED = '\\b(?:non-food|feedstuff|pharmac|medicin|medicinal|pharmaceutical|tobacco|cigarette|coca leaf|coca tea|cannabis|marijuana|magic mushroom|psychoactive|hemp extract|cbd|hashish)\\b'

EXTRA_FOOD = '\\b(?:inflorescence|rhizosphere|amaranthaceous|cruciferous|solanaceous|cucurbit|leguminous|pumpkinaceous|umbelliferous|angiosperm|mammal|mammalian|bovine|ovine|caprine|equine|cervid|camelid|tetrapod|invertebrate|vertebrate|marsupial|lagomorph|carnivore|primate|rodent|fungi|prokaryot|organisms)\\b'

REPLACE = {'bovine': 'cow', 'ovine': 'sheep', 'caprine': 'goat', 'equine': 'horse', 'cervid': 'deer', 'camelid': 'camel', 'mammalian': 'mammalian'}

EXTRA_DISH = '\\b(?:akutaq|akee and saltfish|alimentary paste|baked alaska|babaganoush|bagel chip|bibimbap|bhujia|bhel|ceviche|chicken finger|chicken nugget|chicken wing buffalo|chili con carne|chili sin carne|chili dish|congee|creme brulee|cream vichysoise|crepe|crêpe|croquette|crumpet|daifuku|dolma|enchilada|empanada|escargot bourguignon|escabeche|far far|fish finger|fish stick|fish cake|fishcake|fried rice|fruitcake|fufu|gnocchi|gratin|haleem|halim|hot dog in|hot pocket|hotdog sandwich|khichra|khichdi|kibbe|kofta|kulfi|laksa|macaroon|makowiec|manju|mochi|mousse|muesli bar|muesli|musubi|nachos|ocopa|pakora|panettone|pasta dish|pasta in|pasta with|pate de fruit|peanut brittle|polenta|popcorn ball|quesadilla|raita|ramen dish|rice pudding|sambar|samosa|satay|saute|scalloped|sfogliatelle|sheer khurma|soan papdi|spaghetti bolognese|spaghetti with|stroganoff|succotash|sundae|sweet and sour|tater tot|tempura|toad in|tostada|ugali|wagashi|wonton soup|yokan|yogurt parfait|yoghurt parfait|zongzi)\\b'

EXTRA_INCIDENTAL = '\\b(?:centrifuged|commercial|comminuted|deboned|defatted|decolorized|detoxified|dehulled|dehusked|disintegrated|drained|extruded|heat-treated|high-temperature|homogenized|hydrolyzed|hydrogenated|interesterified|in solution|instantized|micronized|nonfat milk solids|pitted fruit|pre-grilled|pregrilled|protein fortified|radio|recombined|reduced sodium|semi-processed|shelf stable|shelf-stable|soluble|spray-dried|thermally|tube|ultra|unblanched|unwashed|washed|uncored|unpeeled|unsieved|vacuum|vitamin|whitening|wrapping|acidified|alcohol-free|non-alcoholic|nonalcoholic)\\b'

CHEM_PAT = '\\b(?:acetylated|advantame|ammonium|benzoate|bromate|bromide|carboxy|carotene|chlorophyll|chloride|citrate|cyanide|cyclamate|cysteine|dioxide|distarch|ester|esters|ethyl|glutamate|glycerides|glycol|hexylresorcinol|hydrogen|hydroxide|hypochlorite|iodate|isomaltitol|lactate|malate|metabisulphite|metabisulfite|nitrate|nitrite|oxidised|oxidized|oxide|peroxide|phosphate|phosphite|polyphosphate|polysorbate|propylene|propyl|resorcinol|riboflavin|saccharin|silicate|sorbate|stearate|sulphate|sulfate|sulphite|sulfite|tartrate|titanium|tocopherol|triphosphate|xanthophyll)\\b'

def refine_screen(t, context):
    st, _ = initial_screen(t, context)
    target = display_label(t['label'])
    n = normalize(target)
    if st != 'include':
        return (st, target)
    if re.search('^A .+\\.$', target):
        return ('sentence_label', target)
    if len(target) > 80:
        return ('verbose_label', target)
    if not valid_text(target):
        return ('invalid_label', target)
    if re.search(EXTRA_GENERIC, target, re.I) or re.search(EXTRA_MED, target, re.I):
        return ('generic_or_non_cooking', target)
    if re.search('\\b(?:products?|components?|materials?)\\b', target, re.I):
        return ('product_category', target)
    if re.search('\\band\\b|\\bor\\b|/|\\+|\\betc\\b', target, re.I) and (not re.search('^(?:half and half|salt and pepper|half-and-half)$', target, re.I)):
        return ('disjunctive_category', target)
    if re.search(EXTRA_FOOD, target, re.I) and (not re.search('\\b(?:bovine|ovine|caprine|equine|cervid|camelid)\\b', target, re.I)):
        return ('taxonomic_category', target)
    if re.search(EXTRA_DISH, target, re.I) and (not re.search(DISH_EXCEPT, target, re.I)):
        return ('prepared_dish', target)
    if re.search(EXTRA_INCIDENTAL, target, re.I):
        return ('incidental_variant', target)
    if re.search(CHEM_PAT, target, re.I) and n not in COMMON_CHEM:
        return ('industrial_additive', target)
    if re.search('\\b(?:of|from) (?:animal|plant|mammal)|\\b(?:species|spp|sp\\.)\\b', target, re.I):
        return ('taxonomic_category', target)
    if re.search('\\b(?:phylum|genus|genera|cultivar)\\b', target, re.I):
        return ('taxonomic_category', target)
    for old, new in REPLACE.items():
        target = re.sub('\\b' + old + '\\b', new, target, flags=re.I)
    return (st, target)

BASE_EXCLUDE = '^(?:alcoholic beverages?|alcoholic sauce|alcoholic sweet sauce|(?:animal|avian|amphibian|arachnid|insect|crustacean|mollusc|shellfish|seafood|fish meat|freshwater fish|marine fish|groundfish|finfish|fowl|arthropod|gastropod|mammal)(?:s| meat| blood| kidney| liver| fat| external fat| fresh fat tissues| seam fat| lipid| blubber| shortening| broth| bouillon| carcase| paste| refined)?(?:\\b.*)?|nutritional drink|aerated confectionery|confectionery|chemical soft drink|soft drinks?|diet soft drink|water based soft drink|citric acid soft drink|mineral water based soft drink|fermented beverage|distilled fermented beverage|nonfat milk|formulated|prepared|glaze|dry beverage|no sausage casing|no casing|aril spices|aromatic flowers|aromatic herbs|indian pulses|cereal mixes|fats|oils|nuts|seeds|spices|herbs|offal|sugar crops|waxy substance|vegetables|fruits|grain products|pulses|root crops)$'

NOISE = '\\b(?:CIAA|CCFAC|USDA|standard reference|standard for|designation|fat content|starch content|minimum|maximum|more than|less than|less that|less then|not more|obtained|fraction|derivative|derivatives|similar|synthetic|textured protein|hydrolys|hydrolyz|proteins|polyvinyl|tbhq|high acid|low acid|acid-reduced|calcium.reduced|calcium added|protein blend|carbohydrate modified|unstandardized|energy reduced|carbohydrate reduced|low calorie|calorie reduced|calorie-reduced|caloric|perishable|shelf|rpc|for oil production|for industry|for processing|for production|for fish food|for feed|for pet|crystalline|foam|scum|carcase|msm|carcasses|warm-blooded|cold-blooded|amphibians|reptiles|arachnids|snails,|abalones, winkles|eggs,|bird eggs|molluscs|emulsified|emulsions|emulsion|strain.*baby|junior food|weaning|industrial|refined cereal|bitter almond|ornamental)\\b'

PREP = '\\b(?:deviled|devilled|stuffed|breaded|battered|cordial-filled|liquor-filled|cream-filled|creme-filled|with filled|coating from|pre-cooked|cooked, cured|precooked|thermized|reduced alcohol|without alcohol|boiled, frozen|pregrilled|pre-grilled|pumped|pump-cured|immersion cured|structured|dehydrofrozen|coloured|colored|color added|milk solids|sweetener added|microfiltered|finely chopped|cut in|sliced|grated|chopped|julienned|cubed|diced|mashed|minced)\\b'

PREP_KEEP = '\\b(?:canned|cheese|parmesan|cheddar|mozzarella|beef|pork|lamb|veal|venison|chicken|turkey|ham|bacon|meat|olive|sausage|coconut)\\b'

PANTRY = '\\b(?:bread|cracker|biscuit|cookie|ladyfinger|dough|wrapper|pastry|chocolate|candy|marshmallow|caramel|toffee|nougat|praline|gelatin|gelatine|soup mix|stock|broth|sauce|dressing|filling|icing|frosting|topping|cream|milk|cheese|yogurt|yoghurt|jam|jelly|preserve|pasta|noodle|flour|starch|sugar|spice|seasoning|paste|oil|vinegar|juice|fruit|vegetable|bean|pea|lentil|rice|sausage|ham|bacon)\\b'

def screen(t, context):
    st, label = refine_screen(t, context)
    n = normalize(label)
    if st != 'include':
        return (st, label)
    if re.search('\\bfood\\b', label, re.I) and (not re.search('\\b(?:devil.?s food.*mix|food colou?ring)\\b', label, re.I)):
        return ('generic_food_category', label)
    if re.fullmatch(GENERIC_FULL, n) or re.fullmatch(BASE_EXCLUDE, n):
        return ('generic_food_category', label)
    if re.search(NOISE, label, re.I):
        return ('catalog_or_industrial_variant', label)
    if re.search('\\b(?:veal|lamb|beef|pork|chicken|turkey|duck|goose|poultry)\\b', label, re.I) and re.search('\\b(?:cooked|fried|grilled|baked|boiled|sauteed|sautéed|barbequed|barbecued|au jus|roast beef)\\b', label, re.I) and (not re.search('\\b(?:ham|bacon|sausage|canned|stock|broth|fat|dripping)\\b', label, re.I)):
        return ('prepared_meat_dish', label)
    if re.search('\\([^)]*\\bdish\\b', label, re.I):
        return ('prepared_dish', label)
    if re.search(PREP, label, re.I) and (not re.search(PREP_KEEP, label, re.I)):
        return ('recipe_preparation_variant', label)
    if label.count(',') >= 2 and '(' not in label:
        return ('catalog_group_or_formulation', label)
    if re.search('\\b(?:flavour|flavor|flavouring|flavoring|confectionery|beverage base|soft drink base)\\b', label, re.I) and (not re.search('\\b(?:vanilla|almond|orange|lemon|rum|rose|coconut|maple|peppermint|mint)\\b', label, re.I)):
        return ('industrial_flavour_or_group', label)
    definition = ' '.join(t['definitions'])[:160]
    if re.search('\\b(?:is a|is an|a traditional|a popular|a type of|a kind of|a sweet|a savoury|a savory)\\b.*\\b(?:dish|dessert|meal|confection|snack)\\b', definition, re.I) and (not re.search(PANTRY, label, re.I)):
        return ('defined_prepared_dish', label)
    if re.search('\\b(?:syrup|spread|gelatin|jelly|cream|milk|cheese)\\b', label, re.I) and re.search('\\b(?:filled|cold-pack|cold pack|imitation|set|moulded|molded)\\b', label, re.I):
        return ('formulated_variant', label)
    if len(label.split()) == 1 and len(label) < 4 and (n not in {'an', 'egg', 'eel', 'cod', 'fig', 'ham', 'jam', 'oat', 'pea', 'rye', 'yam', 'tea', 'ice'}):
        return ('unclear_short_label', label)
    return (st, label)


BASE_ALIAS_HOLDS = {'pepper', 'pimento', 'malanga', 'cocoyam', 'biscuit', 'cookie', 'cracker'}


def candidate_label(term, context, policy):
    override = policy['overrides'].get(term['id'])
    if override:
        require(term['label'] == override['source_label'], 'Stale cooking override: ' + term['id'])
        if override['action'] == 'hold':
            return 'explicit_hold', display_label(term['label'])
        return 'include', override['label']
    status, label = screen(term, context)
    # Handle inflected dish names and the generic parent catalog independently
    # of the source's uneven singular/plural spelling.
    if status == 'include':
        plural_dish = DISH[:-2] + r'(?:s|es)?\b'
        if re.search(plural_dish, label, re.I) and not re.search(DISH_EXCEPT, label, re.I) and not re.search(KEEP_DISH, label, re.I):
            return 'prepared_dish', label
        if re.search(policy['additional_dish_words'], label, re.I) and not re.search(DISH_EXCEPT, label, re.I):
            return 'prepared_dish', label
        if re.search(policy['additional_hold_words'], label, re.I):
            return 'scope_or_text_hold', label
        if re.search(policy.get('final_category_holds', r'(?!)'), label, re.I):
            return 'final_category_hold', label
        if term['id'].startswith('FOODON_0343'):
            return 'physical_or_dosage_category', label
        if term['id'] in context['additives'] and normalize(label) not in policy['culinary_additives']:
            return 'industrial_additive_scope', label
    if status == 'include' and re.search(r'\b(?:cfr|codex|ccpr|foodex|eurocode|gs1|eurofir)\b', label, re.I):
        return 'classification_residue', label
    if status == 'include' and re.search(r'^\d+\s+-|[<>]|\d+%|\b(?:of which|with added|percent)\b', label, re.I):
        return 'formulation_or_numeric_category', label
    return status, label


def source_rank(term, policy):
    override = policy['overrides'].get(term['id'], {})
    return (term['id'] not in policy['fixed_ids'], override.get('action') != 'include',
            bool(CLASSIFICATION.search(term['label'])), 'food product' in term['label'].lower(),
            not bool(term['definitions']), term['id'])


def prepare(args):
    catalog, policy = load(args.catalog), load(args.policy)
    seed, sources, pilot, descriptor = load(args.seed), load(args.sources), load(args.pilot), load(args.descriptor)
    require(catalog['source'] == descriptor['source'], 'Catalog/descriptor source differs')
    require(policy['source_checksum'] == descriptor['source']['checksum'], 'Cooking policy source differs')
    require(policy['catalog_checksum'] == digest(args.catalog), 'Catalog checksum differs from reviewed selection input')
    require(pilot['base_seed_checksum'] == digest(args.seed) and pilot['base_sources_checksum'] == digest(args.sources),
            'The preserved pilot baseline has changed')
    require(pilot['descriptor_checksum'] == digest(args.descriptor), 'Pilot descriptor differs')
    terms = {t['id']: t for t in catalog['terms']}
    require(len(terms) == len(catalog['terms']), 'Duplicate source IDs')
    context = {'feed': named_descendants(catalog['terms'], 'FOODON_03309997'),
               'hazard': named_descendants(catalog['terms'], 'FOODON_00005146'),
               'additives': named_descendants(catalog['terms'], 'FOODON_03412972'),
               'manual': {f['term_id']: f for f in load(args.findings)['findings']}}
    require(all(tid in terms and terms[tid]['label'] == override['source_label']
                for tid, override in policy['overrides'].items()), 'Stale source-specific override')
    registry = load(args.registry) if args.registry and args.registry.exists() else {'decisions': []}
    known_ids = {d['term_id']: d['canonical_id'] for d in registry['decisions'] if d['action'] == 'import'}
    for tid, eid in policy['fixed_ids'].items():
        require(tid not in known_ids or known_ids[tid] == eid, 'Persisted identity differs from fixed development ID')
        known_ids[tid] = eid
    decisions = copy.deepcopy([d for d in pilot['decisions'] if d['approved']])
    pilot_ids = {d['term_id'] for d in decisions}
    require(len(decisions) == 50 and len(pilot_ids) == 50, 'Expected the preserved 50 pilot mappings')
    entries_by_name = {normalize(e['label']): e for e in seed['entries']}
    # Existing aliases are authored canonical data. Explicit ambiguous words are never identity matches.
    for entry in seed['entries']:
        for alias in entry['aliases']:
            if normalize(alias) not in BASE_ALIAS_HOLDS:
                entries_by_name.setdefault(normalize(alias), entry)
    existing_ids = {e['id'] for e in seed['entries']}
    rows, groups = [], collections.defaultdict(list)
    for term in terms.values():
        if not term['eligible']:
            continue
        status, label = candidate_label(term, context, policy)
        if term['id'] in pilot_ids:
            status = 'preserved_pilot_mapping'
        row = {'term_id': term['id'], 'source_label': term['label'], 'label': label, 'status': status}
        rows.append(row)
        if status == 'include':
            if not valid_text(label) or (term['definitions'] and len(term['definitions'][0]) > 2000):
                row['status'] = 'text_requires_review'
            else:
                groups[normalize(label)].append(term)
    row_by_id = {r['term_id']: r for r in rows}
    # Only collapse a plural spelling when a singular display label is already present.
    # These are additional related source associations, never exact identity equivalences.
    singular = {}
    for key in sorted(groups):
        candidates = []
        if key.endswith('ies'):
            candidates.append(key[:-3] + 'y')
        if key.endswith('oes'):
            candidates.append(key[:-2])
        if key.endswith('leaves'):
            candidates.append(key[:-6] + 'leaf')
        if key.endswith('s') and not key.endswith(('ss', 'us', 'is')):
            candidates.append(key[:-1])
        for candidate in candidates:
            if candidate in groups or candidate in entries_by_name:
                singular[key] = candidate
                break
    for key, target in singular.items():
        # Resolve short chains once, preserving a single display key.
        while target in singular:
            target = singular[target]
        groups[target].extend(groups.pop(key))
    new_imports = []
    for key, candidates in sorted(groups.items()):
        candidates.sort(key=lambda t: source_rank(t, policy))
        winner = candidates[0]
        if key in entries_by_name:
            eid = entries_by_name[key]['id']
            winner = None
        else:
            eid = known_ids.get(winner['id']) or str(uuid.uuid4())
            require(eid not in existing_ids, 'New import UUID already belongs to another ingredient')
            existing_ids.add(eid)
            row = row_by_id[winner['id']]
            override = policy['overrides'].get(winner['id'], {})
            label = row['label']
            # A plural group may have picked a different source spelling; retain the target display spelling.
            target_row = next((row_by_id[t['id']] for t in candidates
                               if normalize(row_by_id[t['id']]['label']) == key), None)
            if target_row:
                label = target_row['label']
            aliases = list(override.get('aliases', []))
            description = winner['definitions'][0] if winner['definitions'] else ''
            entry = {'id': eid, 'label': label, 'aliases': aliases}
            entries_by_name[key] = entry
            decision = dict(approved=True, action='import', term_id=winner['id'], canonical_id=eid,
                            relationship='exact', canonical=dict(label=label, aliases=aliases, description=description),
                            note=f"{policy['version']}; source identity retained under reviewed culinary wording. " + override.get('reason', 'Selected by the cooking/pantry scope policy; source synonyms are not bulk imported.'))
            decisions.append(decision)
            new_imports.append(decision)
            row.update(status='import', canonical_id=eid)
        for term in candidates:
            if winner and term['id'] == winner['id']:
                continue
            # Related is intentionally conservative for cleaned/plural/existing-name overlaps.
            decisions.append(dict(approved=True, action='map', term_id=term['id'], canonical_id=eid,
                                  relationship='related', note=f"{policy['version']}; same culinary display family or maintained canonical name. Related source evidence only; no exact-equivalence inference from strings."))
            row_by_id[term['id']].update(status='related_mapping', canonical_id=eid)
    # Do not copy ambiguous upstream exact synonyms or create conflicting display aliases.
    ownership = collections.defaultdict(set)
    for entry in seed['entries']:
        for name in [entry['label'], *entry['aliases']]:
            ownership[normalize(name)].add(entry['id'])
    for decision in new_imports:
        ownership[normalize(decision['canonical']['label'])].add(decision['canonical_id'])
        for alias in decision['canonical']['aliases']:
            ownership[normalize(alias)].add(decision['canonical_id'])
    dropped_aliases = []
    for decision in new_imports:
        seen, kept = {normalize(decision['canonical']['label'])}, []
        for alias in decision['canonical']['aliases']:
            key = normalize(alias)
            if not valid_text(alias) or key in seen or ownership[key] != {decision['canonical_id']}:
                dropped_aliases.append(dict(term_id=decision['term_id'], alias=alias, reason='duplicate or ambiguous display name'))
                continue
            kept.append(alias)
            seen.add(key)
        decision['canonical']['aliases'] = kept
    # Publication makes UUIDs durable even if later quality checks reject a label.
    # Retain those identities as retired, including their prior mapping evidence.
    # They disappear from suggestions without deleting history or recipe references.
    current_ids = {d['canonical_id'] for d in decisions}
    retired = []
    for old in registry['decisions']:
        if old['action'] == 'import' and old['canonical_id'] not in current_ids:
            retained = copy.deepcopy(old)
            retained['canonical']['status'] = 'retired'
            retained['note'] = f"{policy['version']}; previously staged canonical UUID retained after cooking-scope review. Reason: " + row_by_id[old['term_id']]['status']
            retired.append(retained)
            current_ids.add(old['canonical_id'])
            row_by_id[old['term_id']].update(status='retired_previous_import', canonical_id=old['canonical_id'])
    decisions.extend(retired)
    retired_ids = {d['canonical_id'] for d in retired}
    for old in registry['decisions']:
        if old['action'] == 'map' and old['canonical_id'] in retired_ids:
            decisions.append(copy.deepcopy(old))
    review = dict(schema_version=1, source_checksum=descriptor['source']['checksum'],
                  descriptor_checksum=digest(args.descriptor), base_seed_checksum=digest(args.seed),
                  base_sources_checksum=digest(args.sources), decisions=decisions)
    summary = dict(policy=policy['version'], policy_checksum=digest(args.policy),
                   eligible_terms=len(rows), imports=len(new_imports), active_entries=len(seed['entries']) + len(new_imports),
                   retained_retired_entries=len(retired), total_entries=len(seed['entries']) + len(new_imports) + len(retired),
                   preserved_pilot_mappings=50, related_mappings=sum(d['action'] == 'map' for d in decisions) - 50,
                   dispositions=dict(sorted(collections.Counter(r['status'] for r in rows).items())),
                   dropped_aliases=dropped_aliases,
                   review_method=policy['scope'])
    write_bundle(args.output, {'review.json': review, 'selection.json': rows, 'summary.json': summary})
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--catalog', type=Path, required=True)
    parser.add_argument('--policy', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-cooking-policy.json')
    parser.add_argument('--seed', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-cooking-base-ingredients.json')
    parser.add_argument('--sources', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-cooking-base-sources.json')
    parser.add_argument('--pilot', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-pilot-review.json')
    parser.add_argument('--descriptor', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon.json')
    parser.add_argument('--findings', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-label-review.json')
    parser.add_argument('--registry', type=Path, default=ROOT / 'data/vocabulary/adapters/foodon-cooking-review.json',
                        help='Reuse persisted import UUIDs on subsequent curation runs')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args), indent=2))


if __name__ == '__main__':
    main()
