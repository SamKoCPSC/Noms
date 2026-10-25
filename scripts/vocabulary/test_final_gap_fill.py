"""Final-gap curation, source-identity reuse and explicit retirement contracts."""
import copy
import hashlib
import json
import unittest

import fill_gaps as gaps
import rename_labels
from foodon import normalize,validate


class FinalGapFillTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(gaps.FINAL_GAP_POLICY.read_text())
        cls.seed=json.loads((gaps.ROOT/'data/vocabulary/ingredients.json').read_text())
        cls.sources=json.loads((gaps.ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=gaps.restore_baseline(cls.seed,cls.policy)
        cls.entries={e['id']:e for e in cls.seed['entries']}
        cls.original={e['id']:e for e in cls.base['entries']}
        cls.labels={e['label']:e['id'] for e in cls.seed['entries'] if e['status']=='active'}
        cls.names={normalize(n):e['id'] for e in cls.seed['entries'] if e['status']=='active' for n in [e['label'],*e['aliases']]}

    def test_replay_preserves_every_existing_field_except_explicit_statuses_and_aliases(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        self.assertEqual(gaps.checksum(self.base),'8a6c4f4054700d301dd293daa1de72f3e599723f631fe09eb8be45bee64515ab')
        self.assertEqual((summary['added'],summary['retired_by_review'],summary['active'],summary['retired']),(133,4,13361,85))
        self.assertEqual(validate(self.seed,self.sources),[])
        self.assertEqual(rename_labels.maintained_grocery_stage(self.seed),self.base)
        self.assertEqual(gaps.checksum(self.sources),self.policy['base_sources_checksum'])
        for old in self.base['entries']:
            expected=copy.deepcopy(old)
            expected['aliases']+=self.policy['alias_additions'].get(old['id'],{}).get('aliases',[])
            if old['id'] in self.policy['retirements']:expected['status']='retired'
            self.assertEqual(self.entries[old['id']],expected)

    def test_every_discovery_candidate_and_expansion_check_has_a_review(self):
        r=self.policy['research'];original=r['original_candidates']
        self.assertEqual(len(original),r['original_candidate_names'])
        self.assertEqual(len(original),76)
        self.assertEqual(hashlib.sha256(gaps.encoded(original)).hexdigest(),r['original_candidate_names_sha256'])
        extras=[q for names in r['expansion_groups'].values() for q in names]
        self.assertEqual(len(extras),r['expansion_checklist_names'])
        self.assertEqual(len(extras),154)
        self.assertTrue({normalize(q) for q in [*original,*extras]}<=set(self.policy['reviewed_names']))
        self.assertEqual(len(self.policy['reviewed_names']),r['candidate_names'])
        self.assertEqual(self.policy['context_rows'],{})

    def test_retirement_corrections_keep_historical_records_and_use_correct_existing_ids(self):
        labels={d['before']['label'] for d in self.policy['retirements'].values()}
        self.assertEqual(labels,{'sweet pea','canned sweet pea','Lamb chops','raspberry vinegar'})
        for ident,d in self.policy['retirements'].items():
            expected=copy.deepcopy(d['before']);expected['status']='retired'
            self.assertEqual(self.entries[ident],expected)
        self.assertEqual(self.names['sweet peas'],self.labels['green pea'])
        self.assertEqual(self.names['canned sweet peas'],self.labels['canned green pea'])
        self.assertEqual(self.names['lamb chops'],'167b4335-f244-4b2e-8e99-b8c238af5733')
        self.assertEqual(self.names['raspberry vinegar'],self.labels['raspberry cooking vinegar'])
        self.assertNotEqual(self.names['raspberry vinegar'],'3807bbcf-4d34-4d70-b413-804f258c318f')

    def test_verified_equivalents_reuse_existing_ids_instead_of_adding_duplicates(self):
        for query,target in [('yu choy','choi sum'),('coffee extract','liquid coffee concentrate'),('broccoli stems','broccoli stalk'),
                             ('Malabar spinach','malabar nightshades'),('Vietnamese coriander','vietnamese mint'),
                             ('black trumpet mushrooms','horn of plenty'),('pioppino mushrooms','pioppini mushroom'),
                             ('Chinkiang vinegar','Zhenjiang vinegar'),('banana blossom','banana flowers'),
                             ('prosciutto crudo','prosciutto'),('salt pork','salt pork meat')]:
            ident=self.names[normalize(query)]
            self.assertEqual(ident,self.labels[target])
            self.assertIn(ident,self.original)
            self.assertNotIn(ident,self.policy['entry_reviews'])

    def test_grocery_formulations_and_prepared_parts_remain_distinct(self):
        for labels in [
            ['fat-free beef broth','fat-free reduced-sodium beef broth','reduced-sodium beef broth'],
            ['low-fat cream cheese','reduced-fat cream cheese','nonfat cream cheese'],
            ['plain low-fat Greek yogurt','low-fat Greek yogurt','nonfat plain Greek yogurt'],
            ['fresh lo mein noodles','dried lo mein noodles'],
            ['fresh cavatelli','dried cavatelli'],
            ['fresh Black Mission figs','dried Black Mission figs'],
            ['fresh fines herbes','dried fines herbes','Herbes de Provence'],
            ['ready-to-use chicken consommé','condensed chicken consomme'],
            ['beef sirloin tip','beef sirloin','flat iron steak'],
            ['tofu puffs','aburaage','atsuage','deep-fried tofu'],
        ]:
            self.assertEqual(len({self.labels[n] for n in labels}),len(labels),labels)
        self.assertEqual(self.names['light cream cheese'],self.labels['reduced-fat cream cheese'])

    def test_requested_produce_mushroom_vinegar_and_cured_meat_families_keep_scope(self):
        for labels in [
            ['tatsoi','komatsuna','gai choy','gai lan','choi sum'],
            ['Thai eggplant','turkey berries','Japanese eggplants'],
            ['fresh edible lily bulbs','dried edible lily bulbs','daylily flower buds'],
            ['green shiso leaves','red shiso leaves','perilla leaf'],
            ['porcini mushroom','dried porcini mushrooms','dried mixed mushrooms'],
            ['cremini mushroom','Pholiota chestnut mushrooms','pioppini mushroom'],
            ['unseasoned rice vinegar','seasoned rice vinegar','ume plum vinegar','red rice vinegar','brown rice vinegar'],
            ['tarragon vinegar','tarragon in vinegar'],
            ['Spanish cured chorizo','fresh Mexican chorizo','chorizo'],
            ['guanciale','pork jowl','cured jowl bacon','Garlic bacon'],
            ['lap cheong','lap yuk','Chinese dried duck-liver sausage'],
        ]:
            self.assertEqual(len({self.labels[n] for n in labels}),len(labels),labels)

    def test_extracts_are_not_juice_leaf_syrup_or_oil_aliases(self):
        for labels in [['peppermint extract','mint extract','peppermint'],
                       ['coconut extract','water extract of coconut meat'],
                       ['liquid coffee concentrate','instant espresso powder','espresso'],
                       ['maple extract','maple syrup'],['orange extract','orange juice','Nanfia'],
                       ['pandan extract','pandan leaf'],['ube extract','purple yam','frozen purple yam','ube jam']]:
            self.assertEqual(len({self.labels[n] for n in labels}),len(labels),labels)
        self.assertTrue(all(self.entries[self.labels[n]]['provenance']==dict(origin='curated',references=[]) for n in ['peppermint extract','coconut extract','maple extract']))

    def test_unknown_source_forms_are_not_forced_into_qualified_choices(self):
        holds={q:d for q,d in self.policy['reviewed_names'].items() if d['action']=='hold'}
        self.assertEqual(len(holds),15)
        for q,d in holds.items():
            self.assertIsNone(d['canonical_id'])
            self.assertTrue(d['related_ids'])
            self.assertNotIn(q,self.names,q)
            self.assertTrue(all(self.entries[i]['status']=='active' for i in d['related_ids']))
        for e in self.policy['additions']:
            self.assertEqual(e['source_mappings'],[])
            self.assertEqual(e['provenance'],dict(origin='curated',references=[]))
            review=self.policy['entry_reviews'][e['id']]
            self.assertTrue(review['rationale'] and review['observed_names'] and review['public_evidence'])
            for k in review['public_evidence']:
                d=self.policy['public_evidence'][k]
                self.assertEqual(len(d['sha256']),64)
                self.assertTrue(d['url'].startswith('https://'))

    def test_stale_unexplained_retirements_and_cross_identity_claims_fail(self):
        with self.assertRaisesRegex(ValueError,'Stale'):
            gaps.apply_review(self.seed,self.sources,self.policy)
        p=copy.deepcopy(self.policy);key=next(iter(p['retirements']))
        p['retirements'][key]['reason']=''
        with self.assertRaisesRegex(ValueError,'retirement'):
            gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);p['retirements'][key]['before']['label']='stale source label'
        with self.assertRaisesRegex(ValueError,'retirement'):
            gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);p['additions'][0]['aliases'].append('green pea')
        with self.assertRaisesRegex(ValueError,'shadows'):
            gaps.apply_review(self.base,self.sources,p)
        tampered=copy.deepcopy(self.seed);next(e for e in tampered['entries'] if e['id']==key)['description']='rewritten source history'
        with self.assertRaisesRegex(ValueError,'approved gap output'):
            gaps.restore_baseline(tampered,self.policy)


if __name__=='__main__':unittest.main()
