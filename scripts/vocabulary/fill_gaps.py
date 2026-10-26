"""Apply an explicit cooking-material review to its checksum-pinned preceding seed.

No network, fuzzy identity inference, UUID generation or recipe rewrites.
The ledger owns new UUIDs, reviewed alias suffixes and explicit status retirements.
"""
import argparse
import collections
import copy
import csv
import hashlib
import json
from pathlib import Path

from foodon import normalize, require, valid_id, validate

ROOT=Path(__file__).resolve().parents[2]
DEFAULT_POLICY=ROOT/'data/vocabulary/adapters/ingredient-gap-review.json'
FOLLOWUP_POLICY=ROOT/'data/vocabulary/adapters/ingredient-gap-followup.json'
SECOND_SAMPLE_POLICY=ROOT/'data/vocabulary/adapters/ingredient-second-sample-gaps.json'
GROCERY_POLICY=ROOT/'data/vocabulary/adapters/ingredient-grocery-gaps.json'
FINAL_GAP_POLICY=ROOT/'data/vocabulary/adapters/ingredient-final-gap-fill.json'
NEXT_GAP_POLICY=ROOT/'data/vocabulary/adapters/ingredient-next-gap-fill.json'


def encoded(value):
    return (json.dumps(value,ensure_ascii=False,indent=2)+'\n').encode()


def checksum(value):
    return hashlib.sha256(encoded(value)).hexdigest()


def restore_baseline(result,policy):
    """Recover the exact prior seed from one checksum-pinned curation layer."""
    require(checksum(result)==policy['expected_output_checksum'],'Not the approved gap output')
    additions={e['id']:e for e in policy['additions']}
    require({e['id']:e for e in result['entries'] if e['id'] in additions}==additions,
            'New identities differ from the ledger')
    base=copy.deepcopy(result)
    base['entries']=[e for e in base['entries'] if e['id'] not in additions]
    for e in base['entries']:
        aliases=policy['alias_additions'].get(e['id'],{}).get('aliases',[])
        if aliases:
            require(e['aliases'][-len(aliases):]==aliases,'Existing alias suffix differs from the ledger')
            e['aliases']=e['aliases'][:-len(aliases)]
        retirement=policy.get('retirements',{}).get(e['id'])
        if retirement:
            before=retirement['before']
            require(retirement['reason'].strip() and e=={**before,'status':'retired'},
                    'Retired definition differs from the ledger')
            e['status']=before['status']
    require(checksum(base)==policy['base_seed_checksum'],'Restored baseline differs from approved input')
    return base


def apply_review(seed,sources,policy):
    require(policy['schema_version']==1 and policy['approved'] is True,'Unapproved gap review')
    require(checksum(seed)==policy['base_seed_checksum'] and checksum(sources)==policy['base_sources_checksum'],
            'Stale gap-review baseline')
    require(not validate(seed,sources),'Ambiguous baseline ingredient names')
    result=copy.deepcopy(seed);entries={e['id']:e for e in result['entries']}
    retirements=policy.get('retirements',{})
    for ident,decision in retirements.items():
        require(ident in entries and entries[ident]==decision['before']
                and entries[ident]['status']=='active' and decision['reason'].strip(),
                'Unknown, stale or unexplained retirement')
        entries[ident]['status']='retired'
    names={normalize(n):e['id'] for e in result['entries'] if e['status']=='active'
           for n in [e['label'],*e['aliases']]}
    added=set()

    def claim(name,ident):
        key=normalize(name)
        require(key not in names or names[key]==ident,f'Ingredient name shadows another identity: {name}')
        names[key]=ident

    for entry in policy['additions']:
        ident=entry['id']
        require(valid_id(ident) and ident not in entries,'Duplicate or invalid new UUID')
        require(entry['status']=='active' and entry['provenance']['origin']=='curated' and entry['description'].strip(),
                'New cooking material requires an active curated definition')
        evidence=policy['entry_reviews'].get(ident)
        require(evidence and evidence['rationale'].strip() and evidence['observed_names'],
                'Missing identity review evidence')
        for name in [entry['label'],*entry['aliases']]:claim(name,ident)
        result['entries'].append(copy.deepcopy(entry));entries[ident]=result['entries'][-1];added.add(ident)

    aliases_added=0
    for ident,decision in policy['alias_additions'].items():
        require(ident in entries and ident not in added and entries[ident]['status']=='active'
                and entries[ident]['label']==decision['label'] and decision['note'].strip(),
                'Unknown, retired or stale existing alias target')
        entry=entries[ident];local={normalize(n) for n in [entry['label'],*entry['aliases']]}
        for alias in decision['aliases']:
            claim(alias,ident)
            require(normalize(alias) not in local,'Repeated existing alias proposal')
            entry['aliases'].append(alias);local.add(normalize(alias));aliases_added+=1

    require(len(policy['reviewed_names'])==policy['research']['candidate_names'],'Incomplete gap review')
    for name,decision in policy['reviewed_names'].items():
        require(normalize(name)==name and decision['note'].strip(),'Invalid reviewed recipe name')
        action=decision['action'];ident=decision['canonical_id']
        require(action in {'add','reuse','context','source_error','hold'},'Unknown gap-review action')
        if action=='hold':
            require(ident is None,'Held source must not assign an ingredient')
        else:
            require(ident in entries and entries[ident]['status']=='active','Invalid reviewed target')
            require(action!='add' or ident in added,'Addition decision must name a new identity')
        for related in decision.get('related_ids',[]):
            require(related in entries and entries[related]['status']=='active','Invalid related evidence')

    require(set(policy['entry_reviews'])==added,'Identity reviews must match new UUIDs')
    for key,context in policy.get('context_rows',{}).items():
        require(key.startswith(('mealdb:','yummly:')) and context['status']=='covered'
                and context['note'].strip() and context['canonical_ids'], 'Invalid scoped recipe context')
        require(all(i in entries and entries[i]['status']=='active' for i in context['canonical_ids']),
                'Invalid scoped context target')
    require(not validate(result,sources),'Ambiguous or invalid gap-review output')
    # Existing authored definitions, UUIDs, retired identities and source evidence
    # are immutable except approved alias suffixes and explicit status retirements.
    for original,current in zip(seed['entries'],result['entries']):
        expected=copy.deepcopy(original)
        expected['aliases']+=policy['alias_additions'].get(original['id'],{}).get('aliases',[])
        if original['id'] in retirements:expected['status']='retired'
        require(current==expected,'Gap review rewrote an existing identity')
    summary=dict(added=len(added),aliases_added_to_existing=aliases_added,
                 aliases_on_new_entries=sum(len(e['aliases']) for e in policy['additions']),
                 decisions=dict(collections.Counter(d['action'] for d in policy['reviewed_names'].values())),
                 active=sum(e['status']=='active' for e in result['entries']),
                 retired=sum(e['status']=='retired' for e in result['entries']),output_checksum=checksum(result))
    if retirements:summary['retired_by_review']=len(retirements)
    return result,summary


def run(args):
    policy=json.loads(args.policy.read_text());seed=json.loads(args.seed.read_text());sources=json.loads(args.sources.read_text())
    result,summary=apply_review(seed,sources,policy)
    require(summary['output_checksum']==policy['expected_output_checksum'],'Output differs from reviewed checksum')
    require(not args.output.exists(),'Use a fresh output directory')
    args.output.mkdir(parents=True)
    (args.output/'ingredients.json').write_bytes(encoded(result))
    (args.output/'sources.json').write_bytes(encoded(sources))
    (args.output/'summary.json').write_bytes(encoded(summary))
    rows=[dict(name=name,**d) for name,d in policy['reviewed_names'].items()]
    with (args.output/'decisions.csv').open('w',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=['name','action','canonical_id','note','related_ids'])
        writer.writeheader();writer.writerows(rows)
    print(json.dumps(summary,indent=2))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seed',type=Path,required=True,help='Complete immutable seed matching the selected policy baseline')
    parser.add_argument('--sources',type=Path,required=True)
    parser.add_argument('--policy',type=Path,default=DEFAULT_POLICY)
    parser.add_argument('--output',type=Path,required=True)
    run(parser.parse_args())


if __name__=='__main__':main()
