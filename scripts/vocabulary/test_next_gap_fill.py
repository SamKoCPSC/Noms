"""Next-sample grocery curation, identity boundaries and historical replay."""
import copy
import hashlib
import json
import unittest

import fill_gaps as gaps
import rename_labels
from foodon import normalize, validate


class NextGapFillTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(gaps.NEXT_GAP_POLICY.read_text())
        cls.seed=json.loads((gaps.ROOT/'data/vocabulary/ingredients.json').read_text())
        cls.sources=json.loads((gaps.ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=gaps.restore_baseline(cls.seed,cls.policy)
        cls.entries={e['id']:e for e in cls.seed['entries']}
        cls.original={e['id']:e for e in cls.base['entries']}
        cls.labels={e['label']:e['id'] for e in cls.seed['entries'] if e['status']=='active'}
        cls.names={normalize(n):e['id'] for e in cls.seed['entries'] if e['status']=='active'
                   for n in [e['label'],*e['aliases']]}

    def test_replay_retains_every_prior_definition_and_only_appends_reviewed_aliases(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        self.assertEqual(gaps.checksum(self.base),'0b1bda94a192383da92a95ed6f0834c65024a088a2a0bc4618d43147e0c59f45')
        self.assertEqual((summary['added'],summary['active'],summary['retired']),(118,13479,85))
        self.assertEqual(summary['aliases_added_to_existing'],62)
        self.assertEqual(summary['aliases_on_new_entries'],261)
        self.assertEqual(self.policy['retirements'],{})
        self.assertEqual(rename_labels.maintained_final_gap_stage(self.seed),self.base)
        self.assertEqual(gaps.checksum(self.sources),self.policy['base_sources_checksum'])
        self.assertEqual(validate(self.seed,self.sources),[])
        for old in self.base['entries']:
            expected=copy.deepcopy(old)
            expected['aliases']+=self.policy['alias_additions'].get(old['id'],{}).get('aliases',[])
            self.assertEqual(self.entries[old['id']],expected)

    def test_all_58_candidates_and_every_expansion_choice_have_explicit_reviews(self):
        research=self.policy['research']
        original=research['original_candidates']
        self.assertEqual(len(original),len(set(original)))
        self.assertEqual(len(original),58)
        self.assertEqual(hashlib.sha256(gaps.encoded(original)).hexdigest(),research['original_candidate_names_sha256'])
        extras=[q for group in research['expansion_groups'].values() for q in group]
        self.assertEqual(len(extras),research['expansion_checklist_names'])
        self.assertEqual(len(extras),118)
        self.assertEqual(hashlib.sha256(gaps.encoded(research['expansion_groups'])).hexdigest(),research['expansion_checklist_sha256'])
        self.assertTrue({normalize(q) for q in original+extras}<=set(self.policy['reviewed_names']))
        self.assertEqual(len(self.policy['reviewed_names']),research['candidate_names'])
        self.assertEqual(self.policy['context_rows'],{})

    def test_existing_synonyms_reuse_published_ids(self):
        for query,label in [
            ('thin spaghetti','spaghettini macaroni'),
            ('balsamico bianco','white balsamic vinegar'),
            ('artichoke bottoms','artichoke heart'),
            ('dry-roasted hazelnuts','dry-roasted filbert nut'),
            ('oil-roasted hazelnuts','oil-roasted filbert nut'),
            ('pork blood','pig blood'),('pappadams','poppadum'),
            ('flax seed meal','ground flaxseed'),
            ('sheep casings','sheep intestine casing'),
            ('edible collagen casings','edible collagen casing'),
            ('nonfat evaporated milk','evaporated skim milk'),
            ('spelt milk','spelt drink'),
            ('sweetened tiger nut milk','horchata de chufa'),
        ]:
            ident=self.names[normalize(query)]
            self.assertEqual(ident,self.labels[label])
            self.assertIn(ident,self.original)
            self.assertNotIn(ident,self.policy['entry_reviews'])

    def test_processing_and_formulations_that_change_a_recipe_remain_separate(self):
        for labels in [
            ['tofu skin','fresh tofu skin','dried tofu skin','tofu'],
            ['raw lamb sausage','cooked lamb sausage'],
            ['mentaiko','tarako','cod roe'],
            ['wasabi seasoning powder','pure wasabi powder','wasabi paste','wasabi root'],
            ['plain nonfat yogurt','nonfat yogurt','nonfat plain Greek yogurt','plain low-fat yogurt'],
            ['plain Greek yogurt','plain low-fat Greek yogurt','nonfat plain Greek yogurt'],
            ['veal stock','white veal stock','brown veal stock','veal broth','veal demi-glace'],
            ['canned chicken broth','chicken broth','canned low-sodium chicken broth'],
            ['peanut-only peanut butter','peanut-and-salt peanut butter','unstabilized peanut butter','peanut butter'],
            ['baked crescent rolls','crescent roll dough'],
            ['dried breadcrumbs','plain dried breadcrumbs','fresh breadcrumbs','panko breadcrumbs','seasoned breadcrumbs'],
            ['fresh Calimyrna figs','dried Calimyrna figs','Calimyrna figs'],
            ['Oloroso sherry','Fino sherry','Amontillado sherry','cream sherry wine','Pedro Ximénez sherry'],
            ['white truffle oil','black truffle oil','truffle paste','truffle sauce'],
            ['unsweetened oat milk','barista oat milk','oat milk'],
            ['unsweetened coconut beverage','coconut milk','coconut water'],
            ['coarse sparkling sugar','sanding sugar','granulated sugar'],
        ]:
            self.assertEqual(len({self.labels[label] for label in labels}),len(labels),labels)

    def test_ambiguous_source_wording_offers_choices_without_forced_aliases(self):
        holds={q:d for q,d in self.policy['reviewed_names'].items() if d['action']=='hold'}
        self.assertEqual(len(holds),11)
        for q,d in holds.items():
            self.assertIsNone(d['canonical_id'])
            self.assertNotIn(q,self.names,q)
            self.assertTrue(d['related_ids'])
            self.assertTrue(all(self.entries[i]['status']=='active' for i in d['related_ids']))
            self.assertTrue(any(normalize(name).startswith(q)
                                for ident in d['related_ids']
                                for name in [self.entries[ident]['label'],*self.entries[ident]['aliases']]),q)
        self.assertEqual(set(holds['wasabi powder']['related_ids']),
                         {self.labels['wasabi seasoning powder'],self.labels['pure wasabi powder']})
        self.assertNotIn('spiced amber rum',self.names)
        self.assertNotIn('Chinese rose wine'.lower(),self.names)
        self.assertNotIn('spicy cod roe',self.names)

    def test_less_common_plant_milk_bases_and_tiger_nut_sweetening_stay_distinct(self):
        labels=['hazelnut milk','macadamia milk','walnut milk','pecan milk',
                'Brazil nut milk','flaxseed milk','sesame milk','sunflower seed milk',
                'pumpkin seed milk','quinoa milk','buckwheat milk','amaranth milk',
                'teff milk','millet milk','lupin milk','unsweetened tiger nut milk']
        self.assertEqual(set(labels),set(self.policy['research']['plant_milk_followup']['added_types']))
        self.assertEqual(len({self.labels[label] for label in labels}),16)
        self.assertNotEqual(self.labels['unsweetened tiger nut milk'],self.labels['horchata de chufa'])
        self.assertEqual(self.names['filbert milk'],self.labels['hazelnut milk'])
        self.assertEqual(self.names['linseed milk'],self.labels['flaxseed milk'])
        self.assertEqual(self.names['pepita milk'],self.labels['pumpkin seed milk'])
        self.assertEqual(self.names['lupine milk'],self.labels['lupin milk'])
        for label in labels:
            entry=self.entries[self.labels[label]]
            self.assertEqual(entry['provenance']['origin'],'curated')
            self.assertEqual(entry['source_mappings'],[])
        # Base-only names do not invent an unsweetened or fortified formula.
        for name in ['unsweetened hazelnut milk','unsweetened walnut milk',
                     'fortified sesame milk','tiger nut milk']:
            self.assertNotIn(name,self.names)

    def test_new_definitions_are_curated_without_inferred_source_or_brand_mappings(self):
        self.assertEqual(set(self.policy['entry_reviews']),{e['id'] for e in self.policy['additions']})
        for entry in self.policy['additions']:
            self.assertEqual(entry['source_mappings'],[])
            self.assertEqual(entry['provenance'],dict(origin='curated',references=[]))
            review=self.policy['entry_reviews'][entry['id']]
            self.assertTrue(review['rationale'] and review['definition_basis'] and review['observed_names'])
            self.assertTrue(review['public_evidence'])
            for key in review['public_evidence']:
                evidence=self.policy['public_evidence'][key]
                self.assertTrue(evidence['url'].startswith('https://'))
                self.assertEqual(len(evidence['sha256']),64)
                self.assertTrue(evidence['capture_kind'])

    def test_stale_inputs_and_alias_shadowing_are_rejected(self):
        with self.assertRaisesRegex(ValueError,'Stale'):
            gaps.apply_review(self.seed,self.sources,self.policy)
        policy=copy.deepcopy(self.policy)
        policy['additions'][0]['aliases'].append('soy sauce')
        with self.assertRaisesRegex(ValueError,'shadows'):
            gaps.apply_review(self.base,self.sources,policy)
        changed=copy.deepcopy(self.seed)
        changed['entries'][0]['description']='unreviewed definition rewrite'
        with self.assertRaisesRegex(ValueError,'approved gap output'):
            gaps.restore_baseline(changed,self.policy)


if __name__=='__main__':unittest.main()
