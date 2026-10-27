"""Kitchen curation hides unsuitable suggestions while preserving recipe identities."""
import copy
import json
import unittest

import fill_gaps as gaps
import rename_labels as labels
from foodon import ROOT, normalize, validate


class KitchenReviewTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(labels.KITCHEN_POLICY.read_text())
        cls.seed=json.loads((ROOT/'data/vocabulary/ingredients.json').read_text())
        cls.sources=json.loads((ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=labels.maintained_readable_stage(cls.seed)
        cls.old={e['id']:e for e in cls.base['entries']}
        cls.entries={e['id']:e for e in cls.seed['entries']}
        cls.names={normalize(n):e['id'] for e in cls.seed['entries'] if e['status']=='active'
                   for n in [e['label'],*e['aliases']]}

    def test_replay_and_restore_preserve_frozen_prior_stage_and_validate_current_seed(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        self.assertEqual(gaps.checksum(self.base),'3de52f891231718606912b69dd80eaf4dd304d86ac5ea9b9eb2051599618b483')
        self.assertEqual(labels.restore_baseline(self.base,json.loads(labels.READABLE_POLICY.read_text())),
                         labels.maintained_next_gap_stage(self.seed))
        self.assertEqual(validate(self.seed,self.sources),[])
        self.assertEqual((summary['added'],summary['retired_by_review'],summary['labels_renamed']), (0,21,3))
        self.assertEqual((summary['active'],summary['retired']),(13511,106))
        self.assertEqual(sum(len(e['aliases']) for e in self.seed['entries'] if e['status']=='active'),10088)
        self.assertEqual(gaps.checksum(self.sources),self.policy['base_sources_checksum'])

    def test_exactly_the_24_reviewed_entries_change_without_deleting_or_reassigning_ids(self):
        renamed=set(self.policy['label_renames']);retired=set(self.policy['retirements'])
        self.assertFalse(renamed & retired)
        self.assertEqual(len(renamed|retired),24)
        self.assertEqual(set(self.policy['reviewed_names']),{normalize(self.old[i]['label']) for i in renamed|retired})
        self.assertEqual([e['id'] for e in self.base['entries']],[e['id'] for e in self.seed['entries']])
        for ident,old in self.old.items():
            expected=copy.deepcopy(old)
            if ident in retired:expected['status']='retired'
            if ident in renamed:
                expected['label']=self.policy['label_renames'][ident]['label']
                expected['aliases']+=self.policy['alias_additions'][ident]['aliases']
            self.assertEqual(self.entries[ident],expected)

    def test_retired_feed_classification_and_redundant_names_cannot_be_selected(self):
        for ident,review in self.policy['retirements'].items():
            old=self.old[ident]
            self.assertEqual(self.entries[ident]['status'],'retired')
            self.assertTrue(review['reason'].strip())
            self.assertEqual(review['before'],old)
            # Historical names are not silently moved to a different active UUID.
            for name in [old['label'],*old['aliases']]:self.assertNotIn(normalize(name),self.names)
            for target in review['retained_alternatives']:
                self.assertEqual(self.entries[target]['status'],'active')
                self.assertEqual(self.entries[target],self.old[target])
        self.assertEqual(sum(d['category']=='outside_kitchen_scope' for d in self.policy['retirements'].values()),13)
        for name in ['brewers grains (exhausted)','corn for degermination',
                     'U.S. code of federal regulations, title 21 type']:
            self.assertNotIn(normalize(name),self.names)
        for name in ['dextrose monohydrate','lettuce head','sweet potato','red variety sweet potato',
                     'meat spread','canned meat','boneless chicken thigh','boneless skinless chicken thigh']:
            self.assertIn(name,self.names)

    def test_natural_names_and_legacy_aliases_keep_their_own_kitchen_meaning(self):
        for old,new in [('shiu my pastry case','shumai wrappers'),
                        ('liver based spreadable-textured specialities','liver spread'),
                        ('nut meat','peeled nuts')]:
            ident=next(i for i,e in self.old.items() if e['label']==old)
            self.assertEqual(self.names[normalize(new)],ident)
            self.assertEqual(self.names[normalize(old)],ident)
            self.assertEqual(self.entries[ident]['description'],self.old[ident]['description'])
        self.assertEqual(self.names['shumai wrapper'],self.names['siu mai wrappers'])
        self.assertEqual(len({self.names[n] for n in ['shumai wrappers','wonton wrappers','spring roll wrappers']}),3)
        self.assertNotEqual(self.names['peeled nuts'],self.names['shelled nuts'])
        self.assertNotEqual(self.names['liver spread'],self.names['meat spread'])

    def test_stale_retirements_collisions_and_unreviewed_changes_are_rejected(self):
        with self.assertRaisesRegex(ValueError,'Stale'):
            gaps.apply_review(self.seed,self.sources,self.policy)
        p=copy.deepcopy(self.policy);next(iter(p['retirements'].values()))['before']['description']='changed'
        with self.assertRaisesRegex(ValueError,'retirement'):
            gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);next(iter(p['retirements'].values()))['reason']=''
        with self.assertRaisesRegex(ValueError,'retirement'):
            gaps.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy);next(iter(p['label_renames'].values()))['label']='wonton wrappers'
        with self.assertRaisesRegex(ValueError,'shadows'):
            gaps.apply_review(self.base,self.sources,p)
        changed=copy.deepcopy(self.seed);changed['entries'][0]['description']='unreviewed edit'
        with self.assertRaisesRegex(ValueError,'approved gap output'):
            gaps.restore_baseline(changed,self.policy)


if __name__=='__main__':unittest.main()
