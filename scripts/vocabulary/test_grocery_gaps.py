"""Retail material distinctions and immutable-stage preservation regressions."""
import copy
import json
import unittest

import fill_gaps as gaps
import rename_labels
from foodon import normalize, validate


class GroceryGapsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(gaps.GROCERY_POLICY.read_text())
        cls.seed=json.loads((gaps.ROOT/'data/vocabulary/ingredients.json').read_text())
        cls.sources=json.loads((gaps.ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=gaps.restore_baseline(cls.seed,cls.policy)
        cls.entries={e['id']:e for e in cls.seed['entries']}
        cls.labels={e['label']:e['id'] for e in cls.seed['entries'] if e['status']=='active'}
        cls.names={normalize(n):e['id'] for e in cls.seed['entries'] if e['status']=='active'
                   for n in [e['label'],*e['aliases']]}

    def test_pinned_replay_preserves_every_published_field_and_source(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        self.assertEqual(gaps.checksum(self.base),'6e703364b9ab3b7476a0637b7d1063dcdf0096dfb26d6b7a42b740b39acfa821')
        self.assertEqual(validate(self.seed,self.sources),[])
        self.assertEqual((summary['added'],summary['active'],summary['retired']),(29,13232,81))
        self.assertEqual(rename_labels.maintained_second_sample_stage(self.seed),self.base)
        for old in self.base['entries']:
            expected=copy.deepcopy(old)
            expected['aliases']+=self.policy['alias_additions'].get(old['id'],{}).get('aliases',[])
            self.assertEqual(self.entries[old['id']],expected)
        self.assertEqual(gaps.checksum(self.sources),self.policy['base_sources_checksum'])

    def test_pasta_and_bouillon_forms_are_not_collapsed(self):
        for labels in [
            ['dried angel hair pasta','fresh angel hair pasta'],
            ['chicken bouillon mix','chicken bouillon cube','chicken bouillon paste','concentrated chicken bouillon','chicken stock base with fat removed'],
            ['granulated beef bouillon','beef bouillon cube','beef bouillon paste'],
            ['vegetable bouillon cube','vegetable protein bouillon cube','vegetable stock powder','vegetable bouillon paste','vegetable stock'],
            ['shrimp bouillon','shrimp bouillon cube','granulated shrimp bouillon','powdered shrimp'],
            ['tomato chicken bouillon cube','granulated tomato chicken bouillon','chicken bouillon cube'],
            ['reduced-sodium granulated chicken bouillon','chicken bouillon mix'],
        ]:
            self.assertEqual(len({self.labels[x] for x in labels}),len(labels),labels)
        self.assertEqual(self.names['angel hair'],self.labels['dried angel hair pasta'])
        self.assertEqual(self.names['fresh capellini'],self.labels['fresh angel hair pasta'])
        self.assertEqual(self.names['chicken bouillon powder'],'f2ae6be0-1028-4348-baa7-afe03c2d943d')
        self.assertNotIn(self.names['chicken bouillon powder'],self.policy['entry_reviews'])
        self.assertEqual(self.names['vegetable bouillon powder'],self.labels['vegetable stock powder'])

    def test_corn_processing_and_regional_starch_names_stay_distinct(self):
        labels=['corn flour','corn starch','maize flour','masa harina','white masarepa','yellow masarepa',
                'whole-grain corn flour','whole grain white corn flour','polenta cornmeal','instant polenta','cooked polenta']
        self.assertEqual(len({self.labels[x] for x in labels}),len(labels))
        self.assertEqual(self.names['cornflour'],self.labels['corn starch'])
        self.assertEqual(self.names['US corn flour'.lower()],self.labels['corn flour'])
        self.assertEqual(self.names['precooked white corn flour'],self.labels['white masarepa'])
        self.assertEqual(self.names['nixtamalised maize flour'],self.labels['masa harina'])

    def test_spirits_and_liqueurs_do_not_gain_ambiguous_fruit_aliases(self):
        labels=['brandy','grape brandy','apple brandy','pear brandy','cognac','armagnac','Calvados',
                'pisco','grappa','grappa invecchiata','slivovitz','kirsch','apricot eau-de-vie','apricot liqueur']
        self.assertEqual(len({self.labels[x] for x in labels}),len(labels))
        self.assertEqual(self.names['dry apricot brandy'],self.labels['apricot eau-de-vie'])
        self.assertEqual(self.names['apricot brandy liqueur'],self.labels['apricot liqueur'])
        self.assertNotIn('apricot brandy',self.names)
        self.assertNotIn('cherry brandy',self.names)

    def test_retail_mixtures_and_preparation_names_preserve_material_scope(self):
        for labels in [['coleslaw mix','coleslaw dressing','broccoli slaw mix'],
                       ['harissa','harissa spice blend'],['espresso','instant espresso powder','instant coffee powder'],
                       ['grainy mustard','mustard (dijon)','whole-grain Dijon mustard'],
                       ['italian sausage','sweet Italian sausage','hot Italian sausage'],
                       ['graham crackers','graham flour','breadcrumbs'],['spring mix','mixed greens']]:
            self.assertEqual(len({self.labels[x] for x in labels}),len(labels),labels)
        self.assertEqual(self.names['graham cracker crumbs'],self.labels['graham crackers'])
        self.assertNotIn('broccoli slaw',self.names)
        self.assertNotIn('chicken base',self.names)
        self.assertEqual(self.policy['context_rows'],{})

    def test_every_new_definition_has_reviewed_evidence_without_invented_source_mappings(self):
        self.assertEqual(set(self.policy['entry_reviews']),{e['id'] for e in self.policy['additions']})
        for e in self.policy['additions']:
            self.assertEqual(e['source_mappings'],[])
            self.assertEqual(e['provenance'],dict(origin='curated',references=[]))
            review=self.policy['entry_reviews'][e['id']]
            self.assertTrue(review['rationale'] and review['definition_basis'] and review['public_evidence'])
            for key in review['public_evidence']:
                d=self.policy['public_evidence'][key]
                self.assertTrue(d['url'].startswith('https://'))
                self.assertEqual(len(d['sha256']),64)
        self.assertEqual(self.policy['research']['candidate_names'],len(self.policy['reviewed_names']))

    def test_stale_inputs_and_cross_identity_aliases_are_rejected(self):
        with self.assertRaisesRegex(ValueError,'Stale'):
            gaps.apply_review(self.seed,self.sources,self.policy)
        p=copy.deepcopy(self.policy)
        p['additions'][0]['aliases'].append('fresh angel hair pasta')
        with self.assertRaisesRegex(ValueError,'shadows'):
            gaps.apply_review(self.base,self.sources,p)
        s=copy.deepcopy(self.seed);s['entries'][0]['aliases'].append('unreviewed name')
        with self.assertRaisesRegex(ValueError,'approved gap output'):
            gaps.restore_baseline(s,self.policy)


if __name__=='__main__':unittest.main()
