"""Offline alias and ingredient-scope overlay on an immutable canonical seed.

Every active entry is screened. Approved lexical rules preserve the whole food
name; semantic alternatives require explicit per-identity decisions. Collisions
are held, never resolved by iteration order. Explicit scope retirements preserve
all UUIDs and wording for history. No runtime fuzzy matching or stemming.
"""
import argparse
import collections
import copy
import csv
import hashlib
import json
from pathlib import Path
import re
import unicodedata


def norm(text):
    return ' '.join(unicodedata.normalize('NFKC', text).lower().split())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()


def valid_text(text):
    return isinstance(text, str) and bool(text.strip()) and len(text) <= 200 and not any(
        unicodedata.category(c) == 'Cc' for c in text)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def variants(text, policy):
    """Small, explicitly approved lexical transformations; no qualifier removal."""
    plain = ''.join(c for c in unicodedata.normalize('NFD', text)
                    if not unicodedata.combining(c)).translate(str.maketrans({'œ':'oe','Œ':'Oe','æ':'ae','Æ':'Ae'}))
    if plain != text:
        yield plain, 'accent-spelling'
    unhyphenated = re.sub(r'(?<=[A-Za-z])[‐‑–-](?=[A-Za-z])', ' ', text)
    if unhyphenated != text:
        yield unhyphenated, 'hyphen-spacing'
    for old, replacements in policy['spelling_tokens'].items():
        for new in replacements:
            changed = re.sub(r'(?<![\w-])'+re.escape(old)+r'(?![\w-])', new, text, flags=re.I)
            if changed != text:
                yield changed, 'regional-spelling'
    match = re.fullmatch(r'([^(),]+) \(([^()]+)\)', text)
    if match and norm(match[2]) in policy['prefix_qualifiers']:
        yield match[2]+' '+match[1], 'qualified-word-order'
    if match and norm(match[2]) in policy.get('qualifier_prefixes', {}):
        yield policy['qualifier_prefixes'][norm(match[2])]+' '+match[1], 'scoped-part-word-order'
    # The category and every type/qualifier remain present.
    match = re.fullmatch(r'([^(),]+), ([^(),]+)', text)
    if match and norm(match[1]) in policy['inverted_heads']:
        yield match[2]+' '+match[1], 'inverted-word-order'
    # Inflect only an allowlisted final count noun, leaving parentheses intact.
    # No singularization of arbitrary words (molasses, bass, couscous, rice, ...).
    match = re.fullmatch(r'([^(),/]+?)( \([^()]+\))?', text)
    if match and not re.search(r'\b(and|or)\b', match[1], re.I):
        stem, suffix = match[1], match[2] or ''
        head = stem.split()[-1]
        forms = policy['count_nouns']
        inflected = forms.get(norm(head))
        if inflected is None:
            inverse = {v:k for k,v in forms.items()}
            inflected = inverse.get(norm(head))
        if inflected:
            yield stem[:-len(head)]+inflected+suffix, 'count-noun-number'


def expand(entry, policy):
    roots = [(entry['label'], 'canonical-label'), *[(a, 'existing-alias') for a in entry['aliases']]]
    roots += [(a, 'explicit-editorial') for a in policy['entry_aliases'].get(entry['id'], {}).get('aliases', [])]
    found = {}
    queue = collections.deque((text, reason, 0) for text,reason in roots)
    while queue:
        text, reason, depth = queue.popleft()
        key = norm(text)
        if key in found:
            continue
        require(valid_text(text), f'Invalid alias proposal for {entry["id"]}: {text!r}')
        found[key] = dict(alias=text, reason=reason)
        require(len(found) <= 150, f'Excessive variant expansion: {entry["label"]}')
        if depth < 4:
            for alternative, rule in variants(text, policy):
                if rule not in policy.get('entry_rule_holds', {}).get(entry['id'], {}).get('rules', []):
                    queue.append((alternative, reason+';'+rule, depth+1))
    return found


def enrich(seed, policy):
    require(policy['schema_version'] == 1, 'Unknown alias-policy version')
    require(len(set(policy['count_nouns'].values())) == len(policy['count_nouns']), 'Ambiguous plural lexicon')
    entries = {e['id']:e for e in seed['entries']}
    require(len(entries)==len(seed['entries']), 'Duplicate seed UUID')
    scope = policy.get('scope_review', {})
    retirements = scope.get('retirements', {})
    retained = scope.get('retained', {})
    require(not (set(retirements) & set(retained)), 'Conflicting scope decisions')
    for id, decision in {**retirements, **retained}.items():
        require(id in entries and entries[id]['status']=='active'
                and decision['label']==entries[id]['label'] and decision['note'].strip(),
                f'Unknown, retired or stale scope target: {id}')
    for id, decision in retirements.items():
        require(set(decision)=={'label','category','note','source_mapping'} and decision['category'].strip()
                and decision['source_mapping'].get('relationship')=='exact'
                and decision['source_mapping'] in entries[id]['source_mappings'],
                f'Invalid scope evidence: {id}')
    for id, decision in policy.get('entry_rule_holds', {}).items():
        require(id in entries and decision['note'].strip() and decision['rules'], 'Invalid per-entry rule hold')
    for id, decision in policy['entry_aliases'].items():
        require(id in entries and entries[id]['status']=='active', f'Unknown or retired alias target: {id}')
        require(set(decision)=={'label','aliases','note'} and decision['label']==entries[id]['label']
                and decision['note'].strip() and all(valid_text(a) for a in decision['aliases']), f'Invalid editorial decision: {id}')
    owners = collections.defaultdict(set)
    for e in entries.values():
        if e['status']=='active':
            for text in [e['label'], *e['aliases']]:
                owners[norm(text)].add(e['id'])
    require(all(len(ids)==1 for ids in owners.values()), 'Resolve existing active name collisions before alias enrichment')
    proposals = {}
    candidates = collections.defaultdict(set)
    for e in entries.values():
        if e['status']=='active':
            proposals[e['id']] = expand(e, policy)
            for key in proposals[e['id']]:
                candidates[key].add(e['id'])
    blocked = {norm(k):v for k,v in policy['blocked_aliases'].items()}
    reps = {norm(k):v for k,v in policy['representatives'].items()}
    for key, decision in reps.items():
        require(set(decision)=={'canonical_id','note'} and decision['note'].strip(), 'Invalid alias representative')
        require(decision['canonical_id'] in candidates[key], f'Representative is not a candidate for {key}')
        require(not owners.get(key) or owners[key]=={decision['canonical_id']}, f'Representative would shadow an existing identity: {key}')
    result = copy.deepcopy(seed)
    audit, decisions = [], []
    totals = collections.Counter()
    for e in result['entries']:
        current = {norm(n) for n in [e['label'],*e['aliases']]}
        additions = []
        held = 0
        if e['status']=='active':
            for key, item in sorted(proposals[e['id']].items()):
                if key in current:
                    continue
                if key in blocked:
                    status, note = 'hold_ambiguous', blocked[key]
                elif owners.get(key) and owners[key]!={e['id']}:
                    status, note = 'hold_existing_identity', 'Reserved label/alias belongs to another active UUID.'
                elif len(candidates[key])>1 and reps.get(key,{}).get('canonical_id')!=e['id']:
                    status, note = 'hold_multiple_candidates', 'Multiple identities propose this name; no approved representative for this target.'
                else:
                    status, note = 'add', policy['entry_aliases'].get(e['id'],{}).get('note','Reviewed lexical rule; all food-name qualifiers retained.')
                    additions.append(item['alias'])
                totals[status] += 1
                held += status!='add'
                decisions.append(dict(canonical_id=e['id'], label=e['label'], **item, status=status, note=note))
        e['aliases'] += additions
        require(len(e['aliases'])<=100, f'Alias limit exceeded: {e["label"]}')
        flags = [name for name, pattern in scope.get('label_flags', {}).items() if re.search(pattern, e['label'], re.I)]
        if e['id'] in retirements:
            e['status'] = 'retired'
        scope_action = ('previously_retired' if entries[e['id']]['status']=='retired' else 'retire' if e['id'] in retirements
                        else 'retain_reviewed' if flags or e['id'] in retained else 'no_scope_finding')
        scope_note = retirements.get(e['id'], retained.get(e['id'], {})).get('note',
            'Keyword/context screened; recognizable culinary material or pantry preparation retained.' if flags
            else 'No finding under the recorded lexical screen; not individual semantic certification.')
        audit.append(dict(canonical_id=e['id'], label=e['label'], status=e['status'], existing_aliases=len(entries[e['id']]['aliases']),
            added_aliases=len(additions), held_proposals=held, scope_flags=';'.join(flags), scope_action=scope_action, scope_note=scope_note,
            outcome='retired_scope_review' if e['id'] in retirements else 'retired_unchanged' if e['status']!='active'
                    else 'aliases_added' if additions else 'screened_no_safe_addition'))
    for before, after in zip(seed['entries'], result['entries']):
        expected_status = 'retired' if before['id'] in retirements else before['status']
        require(after['status']==expected_status and before=={**after,'aliases':before['aliases'],'status':before['status']},
                'Overlay changed a field outside aliases or an approved retirement')
        if before['status']=='retired':
            require(before==after, 'Previously retired entry changed')
    # A final global pass protects against ambiguous additions, including ones
    # independently generated for two entries without an existing label match.
    final = {}
    for e in result['entries']:
        if e['status']=='active':
            for text in [e['label'],*e['aliases']]:
                key = norm(text)
                require(key not in final or final[key]==e['id'], f'Active alias collision: {text}')
                final[key] = e['id']
    summary = dict(entries_screened=len(audit), active_screened=len(proposals), retired_unchanged=len(audit)-len(proposals),
        entries_enriched=sum(bool(r['added_aliases']) for r in audit), added_aliases=totals['add'],
        held_proposals=sum(v for k,v in totals.items() if k!='add'), dispositions=dict(totals),
        scope_flagged=sum(bool(r['scope_flags']) for r in audit if r['canonical_id'] in proposals),
        entries_retired=len(retirements), active_final=sum(e['status']=='active' for e in result['entries']),
        retired_final=sum(e['status']=='retired' for e in result['entries']),
        active_aliases=sum(len(e['aliases']) for e in result['entries'] if e['status']=='active'),
        output_checksum=hashlib.sha256(encoded(result)).hexdigest())
    return result, audit, decisions, summary


def run(args):
    seed, policy = json.loads(args.seed.read_text()), json.loads(args.policy.read_text())
    require(digest(args.seed)==policy['base_seed_checksum'] and digest(args.sources)==policy['base_sources_checksum'], 'Stale alias baseline')
    result, audit, decisions, summary = enrich(seed, policy)
    if args.command=='build':
        require(policy.get('approved') is True and summary['output_checksum']==policy.get('expected_output_checksum'),
                'Alias output is not approved; review generated candidates and pin the expected checksum')
    require(not args.output.exists(), 'Use a fresh external output directory')
    args.output.mkdir(parents=True)
    for name, value in [('ingredients.json',result),('sources.json',json.loads(args.sources.read_text())),
                        ('summary.json',summary),('entry-review.json',audit),('alias-review.json',decisions)]:
        (args.output/name).write_bytes(encoded(value))
    for name, rows in [('entries.csv',audit),('aliases.csv',decisions)]:
        if rows:
            with (args.output/name).open('w', newline='') as f:
                writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
    print(json.dumps(summary,indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('command',choices=['review','build'])
    p.add_argument('--seed',type=Path,required=True)
    p.add_argument('--sources',type=Path,required=True)
    p.add_argument('--policy',type=Path,default=Path('data/vocabulary/adapters/ingredient-alias-policy.json'))
    p.add_argument('--output',type=Path,required=True)
    try:
        run(p.parse_args())
    except (ValueError, KeyError, TypeError) as error:
        p.exit(1,str(error)+'\n')
