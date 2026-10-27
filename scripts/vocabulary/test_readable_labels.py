"""Readable display names preserve identity, old names and immutable prior stages."""
import copy
import json
import unittest

import rename_labels as labels
from fill_gaps import checksum
from foodon import ROOT, normalize, validate


class ReadableLabelTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(labels.READABLE_POLICY.read_text())
        cls.seed=json.loads((ROOT/'data/vocabulary/ingredients.json').read_text())
        cls.sources=json.loads((ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=labels.restore_baseline(cls.seed,cls.policy)
        cls.entries={e['id']:e for e in cls.seed['entries']}
        cls.oldlabels={e['label']:e['id'] for e in cls.base['entries']}
        cls.names={normalize(n):e['id'] for e in cls.seed['entries'] if e['status']=='active'
                   for n in [e['label'],*e['aliases']]}

    def test_exact_replay_restore_and_current_totals(self):
        self.assertEqual(checksum(self.base),'4dbdc76d56668cc99f8ec57d941fe463a5861bc3c53f16202e179617b15b2571')
        self.assertEqual(labels.apply_review(self.base,self.sources,self.policy),self.seed)
        self.assertEqual(labels.maintained_next_gap_stage(self.seed),self.base)
        self.assertEqual(validate(self.seed,self.sources),[])
        self.assertEqual(len(self.policy['changes']),476)
        self.assertEqual(len(self.policy['holds']),1016)
        self.assertEqual(len(self.policy['reviewed_ids']),1492)
        self.assertEqual(sum('promoted_alias_index' in d for d in self.policy['changes'].values()),139)
        self.assertEqual(sum(e['status']=='active' for e in self.seed['entries']),13532)
        self.assertEqual(sum(e['status']=='retired' for e in self.seed['entries']),85)
        self.assertEqual(sum(len(e['aliases']) for e in self.seed['entries'] if e['status']=='active'),10082)

    def test_every_old_name_identity_and_unreviewed_record_is_preserved(self):
        self.assertEqual([e['id'] for e in self.seed['entries']],[e['id'] for e in self.base['entries']])
        for old in self.base['entries']:
            new=self.entries[old['id']]
            self.assertEqual({k:v for k,v in old.items() if k not in {'label','aliases'}},
                             {k:v for k,v in new.items() if k not in {'label','aliases'}})
            self.assertLessEqual(set([old['label'],*old['aliases']]),set([new['label'],*new['aliases']]))
            if old['id'] not in self.policy['changes']:
                self.assertEqual(old,new)
            else:
                self.assertIn(old['label'],new['aliases'])
                for name in [old['label'],*old['aliases'],new['label']]:
                    self.assertEqual(self.names[normalize(name)],old['id'])

    def test_egg_forms_and_other_familiar_names_reuse_the_same_ids(self):
        for old,new in [
            ('hen egg mixed whole, dried','whole chicken egg powder'),
            ('hen egg white, dried','dried chicken egg whites'),
            ('hen egg yolk, dried','dried chicken egg yolks'),
            ('coffee ground, roasted','roasted ground coffee'),
            ('milk powder, full fat','full-fat milk powder'),
            ('cocoa beverage-preparation, powder','cocoa drink powder'),
            ('pate, chicken liver','chicken liver pate'),
            ('yoghurt, goat milk','goat milk yogurt'),
            ('mussel shell off, raw)','raw shelled mussels'),
            ('mints, dry','dried mint'),
            ('spelt flour, medium','semi-refined spelt flour'),
            ('nightshade, black','black nightshade leaves'),
            ('berry (impregnated with sugar)','sweetened berries'),
        ]:
            ident=self.oldlabels[old]
            self.assertEqual(self.entries[ident]['label'],new)
            self.assertEqual(self.names[normalize(old)],ident)
            self.assertEqual(self.names[normalize(new)],ident)
        egg_ids={self.oldlabels[n] for n in ['hen egg mixed whole, dried','hen egg white, dried','hen egg yolk, dried','egg powder']}
        self.assertEqual(len(egg_ids),4)
        self.assertNotEqual(self.names['cocoa drink powder'],self.names['cocoa powder'])

    def test_conflicts_unclear_definitions_and_meaningful_technical_names_stay_unchanged(self):
        for name in ['cheese, brie','juice, apple','milk powder, skimmed','cow, minced meat','cow meat, dried',
                     'cocoa mass','soy protein concentrate','liquid coffee concentrate','refined olive oil',
                     'passion fruit pulp','reconstituted milk','transglutaminase preparation',
                     'U.S. code of federal regulations, title 21 type']:
            ident=self.oldlabels[name]
            self.assertIn(ident,self.policy['holds'])
            self.assertEqual(self.entries[ident]['label'],name)
        for name in ['cheese, brie','juice, apple','milk powder, skimmed']:
            self.assertTrue(self.policy['holds'][self.oldlabels[name]]['reason'].startswith('name_owned_by_another_identity:'))

    def test_missing_decisions_and_invalid_explicit_scope_are_rejected(self):
        p=copy.deepcopy(self.policy)
        p['holds'].pop(next(iter(p['holds'])))
        with self.assertRaisesRegex(ValueError,'exactly one'):
            labels.apply_review(self.base,self.sources,p)
        for bad in [[self.policy['reviewed_ids'][0]]*2,['unknown-id'],
                    [next(e['id'] for e in self.base['entries'] if e['status']=='retired')]]:
            p=copy.deepcopy(self.policy);p['reviewed_ids']=bad
            with self.assertRaisesRegex(ValueError,'unique active IDs'):
                labels.apply_review(self.base,self.sources,p)

    def test_collisions_alias_promotion_and_unreviewed_rewrites_are_rejected(self):
        p=copy.deepcopy(self.policy);d=p['changes'][self.oldlabels['hen egg mixed whole, dried']]
        d['after']='chickpea';d.pop('promoted_alias_index',None)
        with self.assertRaisesRegex(ValueError,'collide'):
            labels.apply_review(self.base,self.sources,p)
        p=copy.deepcopy(self.policy)
        d=next(d for d in p['changes'].values() if 'promoted_alias_index' in d)
        d['promoted_alias_index']+=1
        with self.assertRaisesRegex(ValueError,'promoted alias'):
            labels.apply_review(self.base,self.sources,p)
        changed=copy.deepcopy(self.seed);changed['entries'][0]['description']='unreviewed rewrite'
        with self.assertRaisesRegex(ValueError,'approved label output'):
            labels.restore_baseline(changed,self.policy)


if __name__=='__main__':unittest.main()
