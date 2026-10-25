"""Second-sample ingredient definitions, uncertainty and published-ID regressions."""
import copy
import hashlib
import json
import unittest

import fill_gaps as gaps
import rename_labels
from foodon import normalize, validate


class SecondSampleGapsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(gaps.SECOND_SAMPLE_POLICY.read_text())
        cls.seed=json.loads((gaps.ROOT/'data/vocabulary/ingredients.json').read_text())
        cls.sources=json.loads((gaps.ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=gaps.restore_baseline(cls.seed,cls.policy)
        cls.entries={e['id']:e for e in cls.seed['entries']}
        cls.bylabel={e['label']:e for e in cls.seed['entries']}
        cls.names={normalize(n):e['id'] for e in cls.seed['entries'] if e['status']=='active'
                   for n in [e['label'],*e['aliases']]}

    def test_complete_replay_and_every_existing_field_is_preserved(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        self.assertEqual(gaps.checksum(self.base),'f6f5d4021ac18e7a4897752aa4c31aa9d2957ebf17608d4c6e5353ca0d280e4a')
        self.assertEqual(validate(self.seed,self.sources),[])
        self.assertEqual(summary['added'],148)
        self.assertEqual((summary['active'],summary['retired']),(13203,81))
        for entry in self.base['entries']:
            expected=copy.deepcopy(entry)
            expected['aliases']+=self.policy['alias_additions'].get(entry['id'],{}).get('aliases',[])
            self.assertEqual(self.entries[entry['id']],expected)
        self.assertEqual(rename_labels.maintained_label_stage(self.seed),self.base)
        self.assertEqual(gaps.checksum(self.sources),self.policy['base_sources_checksum'])

    def test_all_original_gap_phrases_and_extra_name_discoveries_have_decisions(self):
        research=self.policy['research'];extra=set(research['additional_name_reviews'])
        original=sorted(set(self.policy['reviewed_names'])-extra)
        self.assertEqual(len(original),150)
        self.assertEqual(hashlib.sha256(gaps.encoded(original)).hexdigest(),research['original_gap_names_sha256'])
        self.assertEqual(len(self.policy['reviewed_names']),research['candidate_names'])
        self.assertEqual(set(self.policy['entry_reviews']),{e['id'] for e in self.policy['additions']})
        for e in self.policy['additions']:
            self.assertEqual(e['provenance'],dict(origin='curated',references=[]))
            self.assertEqual(e['source_mappings'],[])
            review=self.policy['entry_reviews'][e['id']]
            self.assertTrue(review['rationale'] and review['observed_names'] and review['definition_basis'])
            for key in review['public_evidence']:
                evidence=self.policy['public_evidence'][key]
                self.assertTrue(evidence['url'].startswith('https://'))
                self.assertEqual(len(evidence['sha256']),64)

    def test_stock_sodium_dairy_and_preservation_differences_retain_distinct_ids(self):
        groups=[
            ['low-sodium beef broth','reduced-sodium beef broth','low-sodium beef stock','unsalted beef stock'],
            ['fresh mozzarella','mozzarella cheese','low moisture mozzarella cheese','low-fat mozzarella'],
            ['nonfat Greek yogurt','nonfat plain Greek yogurt','low-fat Greek yogurt','greek yogurt'],
            ['frozen banana','banana'],
            ['dried strawberries','freeze-dried strawberry','strawberry'],
            ['whole chicken','whole broiler chicken','stewing hen','chicken meat'],
            ['apricot liqueur','apricot eau-de-vie','brandy'],
            ['ginger juice','ginger cordial syrup'],
            ['caramel sauce','caramel browning sauce'],
            ['pistachio paste','pure pistachio paste','sweetened pistachio paste'],
            ['brioche','fine yeast sweet dough (brioche)'],
        ]
        for names in groups:
            found=[self.bylabel[n]['id'] for n in names]
            self.assertEqual(len(set(found)),len(found),names)
        self.assertIn('fat-free low-sodium chicken broth',self.bylabel)
        self.assertIn('33%',self.bylabel['reduced-sodium ham']['description'])
        self.assertIn('bone',self.bylabel['whole chicken']['description'])

    def test_existing_semantic_matches_reuse_published_ids(self):
        for query,target in [('ancho chili ground pepper','ancho powder'),('light mayonnaise','mayonnaise, low fat'),
                             ('rotini','fusilli'),('chicken bones','chicken bone'),
                             ('tvp','textured vegetable protein'),('soft fresh goat cheese','cheese, chevre frais'),
                             ('unsweetened chocolate','chocolate liquor'),('soppressata','sopressata salami')]:
            d=self.policy['reviewed_names'][query]
            self.assertEqual(d['action'],'reuse')
            self.assertEqual(d['canonical_id'],self.bylabel[target]['id'])
            self.assertEqual(self.names[query],d['canonical_id'])
            self.assertNotIn(d['canonical_id'],self.policy['entry_reviews'])

    def test_ambiguous_names_are_not_unconditional_aliases_of_a_qualified_choice(self):
        expected={'apricot brandy','filipino eggplant','thin pizza crust',
                  '33% less sodium ham','40% less sodium taco seasoning mix'}
        holds={q for q,d in self.policy['reviewed_names'].items() if d['action']=='hold'}
        self.assertEqual(holds,expected)
        for q in holds:
            self.assertIsNone(self.policy['reviewed_names'][q]['canonical_id'])
            self.assertNotIn(q,self.names)
        for q in ['dutch stroop','caramelized sugar sauce']:
            self.assertEqual(self.policy['reviewed_names'][q]['action'],'context')
            self.assertNotIn(q,self.names)
        self.assertEqual(len(self.policy['context_rows']),2)
        self.assertTrue(all(k.startswith(('mealdb:53382:','mealdb:53348:')) for k in self.policy['context_rows']))
        self.assertEqual(self.policy['reviewed_names']['ginger cordial']['action'],'source_error')
        self.assertNotEqual(self.names['ginger cordial'],self.names['ginger juice'])
        for q in ['gelatine leafs','color food green']:
            self.assertNotIn(q,self.names)
            self.assertEqual(self.policy['reviewed_names'][q]['action'],'source_error')

    def test_current_msg_evidence_does_not_reactivate_an_obsolete_source_record(self):
        e=self.bylabel['monosodium glutamate']
        self.assertEqual(self.names['msg'],e['id'])
        self.assertEqual(e['source_mappings'],[])
        self.assertIn('fda-msg',self.policy['entry_reviews'][e['id']]['public_evidence'])
        self.assertIn('www.fda.gov',self.policy['public_evidence']['fda-msg']['url'])
        self.assertTrue(all(m['external_id']!='FOODON_03413165' for row in self.seed['entries']
                            for m in row['source_mappings']))
        self.assertEqual(self.names['bowtie pasta'],self.names['farfalle'])
        self.assertEqual(self.names['cellentani'],self.names['cavatappi'])
        self.assertNotEqual(self.names['cavatappi'],self.names['fusilli'])

    def test_stale_output_and_cross_identity_name_claims_are_rejected(self):
        with self.assertRaisesRegex(ValueError,'Stale'):
            gaps.apply_review(self.seed,self.sources,self.policy)
        p=copy.deepcopy(self.policy);p['additions'][0]['aliases'].append('chickpea')
        with self.assertRaisesRegex(ValueError,'shadows'):
            gaps.apply_review(self.base,self.sources,p)
        tampered=copy.deepcopy(self.seed);tampered['entries'][0]['label']='changed existing label'
        with self.assertRaisesRegex(ValueError,'approved gap output'):
            gaps.restore_baseline(tampered,self.policy)


if __name__=='__main__':unittest.main()
