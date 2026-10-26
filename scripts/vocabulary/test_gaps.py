"""Identity boundaries and complete-seed preservation for manual gap curation."""
import copy
import json
from pathlib import Path
import unittest

import rename_labels
import fill_gaps as gaps
from foodon import normalize


class GapReviewTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(gaps.DEFAULT_POLICY.read_text())
        maintained=rename_labels.restore_maintained_baseline(json.loads((gaps.ROOT/'data/vocabulary/ingredients.json').read_text()))
        # Keep checking the original gap stage at its original immutable pin.
        followup=json.loads(gaps.FOLLOWUP_POLICY.read_text())
        cls.seed=gaps.restore_baseline(maintained,followup)
        cls.sources=json.loads((gaps.ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=gaps.restore_baseline(cls.seed,cls.policy)
        cls.entries={e['label']:e for e in cls.seed['entries']}

    def test_complete_bundle_replays_and_preserves_all_previous_definitions(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        self.assertEqual(len(result['entries']),len(self.base['entries'])+len(self.policy['additions']))
        self.assertEqual(sum(e['status']=='retired' for e in result['entries']),81)
        original={e['id']:e for e in self.base['entries']}
        current={e['id']:e for e in result['entries']}
        for ident,e in original.items():
            expected=copy.deepcopy(e)
            expected['aliases']+=self.policy['alias_additions'].get(ident,{}).get('aliases',[])
            self.assertEqual(current[ident],expected)

    def test_cooking_distinctions_keep_separate_uuids_and_names(self):
        groups=[
            ['chicken stock','chicken broth','chicken bouillon cube','chicken bouillon (concentrated)'],
            ['single cream','double cream','heavy cream','light whipping cream','whipping cream'],
            ['chicken broth','low-sodium chicken broth','reduced-sodium chicken broth','fat-free chicken broth','fat-skimmed chicken broth','fat-free reduced-sodium chicken broth'],
            ['soy sauce','low-sodium soy sauce','reduced-sodium soy sauce','reduced-sodium tamari'],
            ['basmati rice','jasmine rice','rice for sushi','rice for paella'],
            ['corn tortillas','wheat flour tortillas','tostada shells'],
            ['lasagne pasta sheets','cooked lasagne pasta sheets'],
            ['polenta cornmeal','cooked polenta'],
            ['rice for sushi','cooked short-grain rice','seasoned sushi rice'],
            ['sweet soy sauce','kecap manis'],
            ['chicken breast (boneless)','chicken breast (skinless, boneless)'],
            ['poblano pepper','ancho pepper'],
        ]
        for names in groups:
            entries=[self.entries[n] for n in names]
            self.assertEqual(len({e['id'] for e in entries}),len(entries),names)
            owned=[{normalize(n) for n in [e['label'],*e['aliases']]} for e in entries]
            for i,left in enumerate(owned):
                for right in owned[i+1:]:self.assertFalse(left&right,names)

    def test_existing_equivalents_are_reused_instead_of_duplicated(self):
        pairs=[('spaghetti','spaghetti macaroni'),('date molasses','date honey'),
               ('1% low-fat milk','low-fat milk'),('pasta shells','conchiglie'),
               ('japanese soy sauce','shoyu sauce'),('dark molasses','second molasses')]
        for name,label in pairs:
            d=self.policy['reviewed_names'][name]
            self.assertEqual(d['action'],'reuse')
            self.assertEqual(d['canonical_id'],self.entries[label]['id'])

    def test_ambiguous_source_names_do_not_become_unconditional_aliases(self):
        for q in ['polenta','pectin','vanilla essence','ground red pepper','black moss','lily buds']:
            d=self.policy['reviewed_names'][q]
            self.assertEqual(d['action'],'hold',q)
            self.assertIsNone(d['canonical_id'])
        for q in ['grate lime peel','fat free frozen top whip','nori','sushi rice']:
            self.assertIn(self.policy['reviewed_names'][q]['action'],['context','source_error'])
            for e in self.policy['additions']:self.assertNotIn(q,[normalize(a) for a in e['aliases']])
        self.assertNotEqual(self.policy['context_rows']['mealdb:53033:1']['canonical_ids'],
                            self.policy['context_rows']['mealdb:53034:9']['canonical_ids'])
        self.assertNotIn('cassava flatbread',self.entries['casabe']['aliases'])
        self.assertNotIn('thin rice noodles',self.entries['rice vermicelli']['aliases'])

    def test_shadowing_an_existing_material_and_reusing_uuid_fail(self):
        for mutation in ['name','id']:
            p=copy.deepcopy(self.policy)
            if mutation=='name':p['additions'][0]['aliases'].append('chicken broth')
            else:p['additions'][0]['id']=self.base['entries'][0]['id']
            with self.assertRaises(ValueError):gaps.apply_review(self.base,self.sources,p)

    def test_stale_input_missing_review_and_retired_target_fail(self):
        p=copy.deepcopy(self.policy);p['base_seed_checksum']='0'*64
        with self.assertRaisesRegex(ValueError,'Stale'):gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);p['reviewed_names'].pop(next(iter(p['reviewed_names'])))
        with self.assertRaisesRegex(ValueError,'Incomplete'):gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);p['entry_reviews'].pop(p['additions'][0]['id'])
        with self.assertRaisesRegex(ValueError,'evidence'):gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);retired=next(e for e in self.base['entries'] if e['status']=='retired')
        p['alias_additions'][retired['id']]=dict(label=retired['label'],aliases=['unsafe revival'],note='Fixture')
        with self.assertRaisesRegex(ValueError,'retired'):gaps.apply_review(self.base,self.sources,p)


if __name__=='__main__':unittest.main()
