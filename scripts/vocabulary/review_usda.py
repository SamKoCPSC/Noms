#!/usr/bin/env python3
"""Screen every eligible USDA description and retain case-level identity evidence."""
import argparse
import collections
import copy
import csv
import json
from pathlib import Path
import re
import unicodedata
import uuid

from foodon import ROOT, digest, load, normalize, require, valid_text, write_bundle
import usda

SINGULAR = dict(apples='apple', apricots='apricot', bananas='banana', berries='berry',
    beans='bean', beets='beet', blueberries='blueberry', cherries='cherry', carrots='carrot',
    chickpeas='chickpea', cranberries='cranberry', dates='date', eggs='egg', figs='fig',
    grapes='grape', kiwifruit='kiwi', lemons='lemon', lentils='lentil', limes='lime',
    mangoes='mango', melons='melon', mushrooms='mushroom', nuts='nut', onions='onion',
    oranges='orange', peaches='peach', pears='pear', peas='pea', peppers='pepper',
    plums='plum', potatoes='potato', prunes='prune', raspberries='raspberry',
    seeds='seed', shallots='shallot', strawberries='strawberry', sugars='sugar',
    tomatoes='tomato', turnips='turnip', walnuts='walnut', pecans='pecan',
    almonds='almond', hazelnuts='hazelnut', pistachios='pistachio', peanuts='peanut',
    cashews='cashew', brazilnuts='brazil nut', leaves='leaf', cloves='clove',
    blackberries='blackberry', blackeyed='blackeye', collards='collard', sprouts='sprout',
    nectarines='nectarine', parsnips='parsnip', rutabagas='rutabaga', kumquats='kumquat',
    quinces='quince', clementines='clementine', tamarinds='tamarind', longans='longan',
    loquats='loquat', mangos='mango', mulberries='mulberry', papayas='papaya',
    gooseberries='gooseberry', elderberries='elderberry', crabapples='crabapple',
    pomegranates='pomegranate', acorns='acorn', beechnuts='beechnut', hickorynuts='hickorynut',
    butternuts='butternut', pilinuts='pili nut', noodles='noodle', syrups='syrup',
    capers='caper', seed='seed', dry='dried', litchis='lychee', nopales='nopal',
    shoots='shoot', roots='root', flowers='flower', hearts='heart', grains='grain',
    artichokes='artichoke', kernels='kernel',
    hickorynut='hickory nut', tomatillos='tomatillo', jerusalemartichokes='jerusalem artichoke',
    fillets='fillet', filet='fillet', tinned='canned', uncooked='raw', dehydrated='dried',
    yoghurt='yogurt', soy='soybean', garbanzo='chickpea', scallions='scallion')


def signature(text):
    text = ''.join(c for c in unicodedata.normalize('NFKD', text.casefold())
                   if not unicodedata.combining(c))
    text = re.sub(r"[’']s\b", '', text)
    text = text.replace('sunflower seed oil', 'sunflower oil')
    for before, after in [('grapeseed', 'grape seed'), ('tomatoseed', 'tomato seed'),
                          ('teaseed', 'tea seed'), ('mahimahi', 'mahi mahi'),
                          ('jerusalem-artichoke', 'jerusalem artichoke')]:
        text = text.replace(before, after)
    return ' '.join(sorted(part for word in re.findall(r'[a-z0-9]+', text)
                           for part in SINGULAR.get(word, word).split()))


def cleaned_parts(label):
    label = re.sub(r"\s*\(Includes foods for USDA.s Food Distribution Program\)", '', label, flags=re.I)
    parts = [p.strip().lower() for p in label.split(',') if p.strip()]
    if parts and parts[0] in {'spices', 'nuts', 'seeds', 'fish', 'mollusks', 'crustaceans',
                              'beverages', 'alcoholic beverage', 'alcoholic beverages', 'game meat'}:
        parts = parts[1:]
    if parts and parts[0] == 'soup' and len(parts) > 1 and re.search(
            r'\b(?:broth|stock|bouillon|consomme)\b', parts[1]):
        parts = parts[1:]
    return parts


def display_label(parts):
    if not parts:
        return ''
    head, *qualifiers = parts
    head = SINGULAR.get(head, head)
    if head in {'cheese', 'flour', 'oil', 'sugar', 'syrup', 'juice', 'butter', 'sauce'} and qualifiers:
        head = qualifiers.pop(0) + ' ' + head
    return head + (' (' + ', '.join(qualifiers) + ')' if qualifiers else '')


def scope_hold(term, policy):
    label, category = term['label'], term['category_id']
    if category in policy['held_categories']:
        return 'hold_category', policy['held_categories'][category]
    for rule in policy['hold_rules']:
        if re.search(rule['pattern'], label, re.I):
            return rule['status'], rule['note']
    if category == '6' and label.lower().startswith('soup,') and not re.match(
            r'^Soup, (?:(?:beef|chicken|vegetable|fish) )?(?:broth|stock|bouillon|consomme)', label, re.I):
        return 'hold_complete_soup', 'Finished soup rather than a cooking stock, sauce or base.'
    if category == '18' and not re.search(r'\b(?:bread|tortilla|crumbs|breadcrumbs|crust|dough|batter|phyllo|puff pastry|dry mix)\b', label, re.I):
        return 'hold_finished_baked_food', 'Finished baked food outside the initial ingredient scope.'
    if category == '19' and not re.search(r'\b(?:sugar|sugars|syrup|syrups|honey|molasses|cocoa|chocolate|jam|jams|jellies|marmalade|pectin|gelatin|frosting|icing|baking)\b', label, re.I):
        return 'hold_finished_sweet', 'Finished sweet rather than a specific baking/pantry material.'
    if category == '14' and not re.search(r'\b(?:water|coffee|tea|cocoa|milk|wine|beer|sake|juice|vinegar|liquor|rum|whiskey|whisky|gin|vodka|brandy)\b', label, re.I):
        return 'hold_prepared_beverage', 'Prepared beverage or mix outside the initial ingredient scope.'
    return None


def variants(term, policy):
    """Broader simplifications propose related evidence only; never import wording."""
    parts = cleaned_parts(term['label'])
    strict = ' '.join(parts)
    simplified = []
    for part in parts:
        if any(re.fullmatch(pattern, part, re.I) for pattern in policy['association_only_drop_parts']):
            continue
        simplified.append(part)
    names = [strict]
    if simplified != parts:
        names.append(' '.join(simplified))
    if 'uncooked' in simplified:
        names.append(' '.join(p for p in simplified if p != 'uncooked'))
    # Raw material can associate with its generic maintained ingredient, without
    # inferring an exact match. Canned/dried/cooked parts are never dropped here.
    if 'raw' in simplified:
        names.append(' '.join(p for p in simplified if p != 'raw'))
    return parts, list(dict.fromkeys(n for n in names if signature(n)))


def prepare(args):
    descriptor, base, sources, terms, counts = usda.context(args)
    policy = load(args.policy)
    require(policy['schema_version'] == 1 and policy['descriptor_checksum'] == digest(args.descriptor),
            'Policy targets a different USDA descriptor')
    overrides = policy['overrides']
    require(set(overrides) <= set(terms), 'Uncaptured USDA override')
    registry = load(args.registry) if args.registry.exists() else {'decisions': []}
    prior = {d['record_id']: d['canonical_id'] for d in registry['decisions'] if d['action'] == 'import'}
    entries = {e['id']: e for e in base['entries'] if e['status'] == 'active'}
    index = collections.defaultdict(set)
    for eid, entry in entries.items():
        for name in [entry['label'], *entry['aliases']]:
            key = signature(name)
            if key:
                index[key].add(eid)
    rows, planned, groups = [], [], {}
    for rid, term in terms.items():
        parts, names = variants(term, policy)
        row = dict(record_id=rid, fdc_id=term['fdc_id'], label=term['label'],
                   category=term['category'], publication_date=term['publication_date'],
                   ndb_number=term['ndb_number'], record_checksum=term['record_checksum'])
        override = overrides.get(rid)
        hold = scope_hold(term, policy)
        if override:
            decision = dict(copy.deepcopy(override), record_id=rid)
            row.update(status='explicit_' + decision['action'], note=decision['note'])
        elif hold:
            row.update(status=hold[0], note=hold[1]); rows.append(row); continue
        else:
            hits, matched_name = set(), None
            for name in [term['label'], *names]:
                hits = index.get(signature(name), set())
                if not hits:
                    target = policy['association_names'].get(signature(name))
                    if target:
                        require(target in entries, 'Unknown reviewed association target')
                        hits = {target}
                if hits:
                    matched_name = name; break
            if hits:
                direct = {eid for eid in hits if signature(entries[eid]['label']) == signature(matched_name)}
                candidates = direct or hits
                # One representative may receive related evidence for legacy duplicate names.
                if len({signature(entries[eid]['label']) for eid in candidates}) > 1:
                    row.update(status='hold_ambiguous_identity', candidate_ids=sorted(hits),
                               note='Distinct maintained identities match; no label/alias merge inferred.')
                    rows.append(row); continue
                eid = min(candidates, key=lambda k: (entries[k]['provenance']['origin'] != 'curated', k))
                decision = dict(action='map', record_id=rid, canonical_id=eid, relationship='related',
                    note='Scoped description-name association with ' + entries[eid]['label'] +
                         '. USDA record retains its food form, preparation and analytical qualifiers; '
                         'no exact equivalence, alias adoption or nutrient linkage is inferred.')
                row.update(status='map_existing', candidate_ids=sorted(hits), note=decision['note'])
            else:
                # Unmatched analytical/product descriptions stay held. Only concise
                # scope-preserving wording can become a new reviewed recipe name.
                label = display_label(parts)
                key = signature(' '.join(parts))
                analytical = (any(re.search(p, term['label'], re.I) for p in policy['no_import_patterns']) or
                              any(re.search(p, term['label']) for p in policy['case_sensitive_no_import_patterns']))
                if key not in groups and (not policy['allow_import_candidates'] or analytical or
                        len(parts) > policy['max_unmatched_parts'] or not valid_text(label, 120)):
                    row.update(status='hold_unmatched_detail',
                        note='No maintained identity match; analytical, alternative or complex wording needs a specific editorial definition.')
                    rows.append(row); continue
                if key in groups:
                    decision = dict(action='map', record_id=rid, target_record_id=groups[key],
                        relationship='related', note='Repeated reviewed food-name family across FDC records; one canonical UUID, related source evidence only.')
                    row.update(status='map_new_name_family', note=decision['note'])
                else:
                    decision = dict(action='import', record_id=rid, relationship='exact',
                        canonical=dict(label=label, aliases=[], description='USDA food description: ' + term['label']),
                        note='Concise culinary material; display wording retains source parts/forms. Source aliases and nutrient values are not imported.')
                    groups[key] = rid
                    row.update(status='import_candidate', note=decision['note'])
        if decision['action'] == 'import':
            groups[signature(' '.join(parts))] = rid
            groups[signature(decision['canonical']['label'])] = rid
        if decision['action'] != 'hold':
            planned.append(decision)
        rows.append(row)
    ids = {d['record_id']: prior.get(d['record_id']) or str(uuid.uuid4())
           for d in planned if d['action'] == 'import'}
    require(set(prior) <= set(ids), 'Policy drops a persisted USDA import; review retirement explicitly')
    ownership = usda.name_index(base)
    for d in planned:
        if d['action'] == 'import':
            for name in [d['canonical']['label'], *d['canonical']['aliases']]:
                require(not ownership[normalize(name)], 'New USDA label/alias collision: ' + name)
                ownership[normalize(name)].add(ids[d['record_id']])
    decisions = []
    for d in planned:
        target = d.get('target_record_id')
        require(target is None or target in ids, 'Map target must be a planned USDA import')
        eid = ids[d['record_id']] if d['action'] == 'import' else (ids[target] if target else d['canonical_id'])
        require(d['action'] == 'import' or eid in entries or eid in ids.values(), 'Unknown USDA canonical target')
        decision = dict(approved=True, action=d['action'], record_id=d['record_id'],
            record_checksum=terms[d['record_id']]['record_checksum'], canonical_id=eid,
            relationship=d['relationship'], note=policy['version'] + '; ' + d['note'])
        if d['action'] == 'import':
            decision['canonical'] = d['canonical']
        decisions.append(decision)
    decisions.sort(key=lambda d: d['action'] != 'import')
    by_record = {d['record_id']: d for d in decisions}
    for row in rows:
        if row['record_id'] in by_record:
            row.update(canonical_id=by_record[row['record_id']]['canonical_id'],
                       relationship=by_record[row['record_id']]['relationship'])
    matched = collections.defaultdict(list)
    for d in decisions:
        matched[d['canonical_id']].append(d['record_id'])
    names = [dict(canonical_id=eid, label=e['label'], aliases=e['aliases'],
                  records=matched[eid], status='associated' if matched[eid] else 'held_or_unmatched')
             for eid, e in entries.items()]
    summary = dict(records=len(rows), dispositions=dict(sorted(collections.Counter(r['status'] for r in rows).items())),
        imports=len(ids), mappings=len(decisions)-len(ids), active_baseline_names=len(names),
        associated_baseline_names=sum(bool(n['records']) for n in names),
        food_rows_by_type=counts, policy_checksum=digest(args.policy),
        method='Policy-assisted complete name screening with explicit exceptions; holds are not absence claims or semantic certification.')
    write_bundle(args.output, {'review.json': {**usda.bindings(args), 'decisions': decisions},
        'source-name-review.json': rows, 'seed-name-review.json': names, 'summary.json': summary})
    for filename, records, fields in [
        ('source-names.csv', rows, ['record_id','fdc_id','label','category','publication_date','ndb_number','record_checksum','status','canonical_id','relationship','candidate_ids','note']),
        ('seed-names.csv', names, ['canonical_id','label','aliases','status','records'])]:
        with (args.output/filename).open('w', encoding='utf-8-sig', newline='') as stream:
            writer = csv.DictWriter(stream, fieldnames=fields, extrasaction='ignore'); writer.writeheader()
            for row in records:
                writer.writerow({k: ' | '.join(v) if isinstance(v, list) else v for k,v in row.items() if k in fields})
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__); usda.arguments(parser)
    parser.add_argument('--policy', type=Path, default=ROOT/'data/vocabulary/adapters/usda-cooking-policy.json')
    parser.add_argument('--registry', type=Path, default=ROOT/'data/vocabulary/adapters/usda-cooking-review.json')
    print(json.dumps(prepare(parser.parse_args())))


if __name__ == '__main__':
    main()
