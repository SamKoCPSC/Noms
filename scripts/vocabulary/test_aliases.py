"""Alias overlay regressions: identity boundaries, collisions and reproducibility."""
import argparse
import json
from pathlib import Path
import tempfile
import unittest

import review_aliases as aliases


class AliasTest(unittest.TestCase):
    def setUp(self):
        self.policy=json.loads(Path('data/vocabulary/adapters/ingredient-alias-policy.json').read_text())
        self.policy['entry_aliases']={}
        self.policy['representatives']={}
        self.policy['entry_rule_holds']={}

    def entry(self, label, id='one', **kw):
        return dict(id=id,label=label,aliases=[],description='Authored definition.',locale='en',status='active',
                    source_mappings=[],provenance=dict(origin='curated',references=[]),**kw)

    def run_review(self, *entries):
        seed=dict(schema_version=1,dimension='ingredient',entries=list(entries))
        return aliases.enrich(seed,self.policy)

    def names(self, label):
        return {aliases.norm(r['alias']) for r in aliases.expand(self.entry(label),self.policy).values()}

    def test_plurals_and_qualifiers_are_preserved(self):
        self.assertIn('eggs',self.names('egg'))
        self.assertIn('tomatoes',self.names('tomato'))
        self.assertIn('bay leaves',self.names('bay leaf'))
        self.assertIn('mushroom',self.names('mushrooms'))
        forms=self.names('tomato (canned)')
        self.assertIn('canned tomatoes',forms)
        self.assertIn('tomatoes (canned)',forms)
        self.assertNotIn('tomatoes',forms)
        self.assertNotIn('fresh tomato',forms)

    def test_mass_nouns_and_invariant_names_are_not_blindly_stemmed(self):
        for label in ['rice','molasses','couscous','bass','grass','water','cream','milk','ground beef','Swiss cheese']:
            forms=self.names(label)
            self.assertFalse(any('count-noun-number' in r['reason'] for r in aliases.expand(self.entry(label),self.policy).values()),label)
        self.assertNotIn('salt and peppers',self.names('salt and pepper'))

    def test_accents_spelling_and_inversion_do_not_strip_materials(self):
        self.assertIn('creme fraiche',self.names('crème fraîche'))
        self.assertIn('fresh red chilli peppers',self.names('red chili pepper (fresh)'))
        self.assertIn('goat cheese',self.names('cheese, goat'))
        self.assertNotIn('goat',self.names('cheese, goat'))
        self.assertNotIn('milk',self.names('milk (powdered)'))
        self.assertNotIn('pepper',self.names('peppercorn'))
        self.assertIn('bone-in lamb chop',self.names('lamb chop (with bone)'))
        self.assertNotIn('boneless lamb chop',self.names('lamb chop (with bone)'))
        self.assertIn('shelled walnut',self.names('walnut (shell off)'))

    def test_maintained_seed_is_the_approved_alias_output(self):
        policy=json.loads(Path('data/vocabulary/adapters/ingredient-alias-policy.json').read_text())
        seed_path=Path('data/vocabulary/ingredients.json')
        self.assertIs(policy['approved'],True)
        self.assertEqual(aliases.digest(seed_path),policy['expected_output_checksum'])
        self.assertEqual(aliases.digest(Path('data/vocabulary/sources.json')),policy['base_sources_checksum'])
        seed=json.loads(seed_path.read_text());by_label={e['label']:e for e in seed['entries']}
        for label,alias in [('egg','eggs'),('all-purpose flour','plain flour'),('cumin (powdered)','ground cumin'),
                            ('eggplant','egg plants'),('coriander leaf','cilantro leaves')]:
            self.assertIn(alias,by_label[label]['aliases'])
        for label,forbidden in [('heavy cream','double cream'),('corn starch','corn flour'),
                                 ('black pepper','pepper'),('chili (canned)','canned chilies')]:
            self.assertNotIn(forbidden,by_label[label]['aliases'])

    def test_every_entry_is_screened_and_only_aliases_change(self):
        original=self.entry('egg');retired=self.entry('old tomato','retired');retired['status']='retired'
        out,rows,decisions,summary=self.run_review(original,retired)
        self.assertEqual(summary['active_screened'],1)
        self.assertEqual(summary['retired_unchanged'],1)
        self.assertEqual(summary['entries_enriched'],1)
        self.assertEqual(out['entries'][1],retired)
        self.assertEqual(original,{**out['entries'][0],'aliases':[]})
        self.assertEqual(rows[1]['outcome'],'retired_unchanged')
        self.assertEqual(self.run_review(original,retired)[0],out)

    def test_original_alias_order_is_retained(self):
        original=self.entry('garbanzo bean');original['aliases']=['chickpea','garbanzo']
        result,*_=self.run_review(original)
        self.assertEqual(result['entries'][0]['aliases'][:2],original['aliases'])

    def test_existing_names_are_never_shadowed(self):
        out,_,decisions,_=self.run_review(self.entry('egg'),self.entry('eggs','two'))
        self.assertEqual(out['entries'][0]['aliases'],[])
        self.assertEqual(out['entries'][1]['aliases'],[])
        self.assertTrue(all(r['status']=='hold_existing_identity' for r in decisions))

    def test_candidate_collisions_are_held_independent_of_entry_order(self):
        first=self.entry('apple (dried)');second=self.entry('dried apples','two')
        forward=self.run_review(first,second)
        reverse=self.run_review(second,first)
        self.assertEqual({e['id']:e for e in forward[0]['entries']},{e['id']:e for e in reverse[0]['entries']})
        self.assertTrue(any(r['alias']=='dried apple' and r['status']=='hold_multiple_candidates' for r in forward[2]))

    def test_reviewed_representative_retains_both_uuids(self):
        first=self.entry('apple (dried)');second=self.entry('dried apples','two')
        self.policy['representatives']['dried apple']=dict(canonical_id='two',note='Reviewed same preserved apple material.')
        result,*_=self.run_review(first,second)
        self.assertIn('dried apple',result['entries'][1]['aliases'])
        self.assertEqual([e['id'] for e in result['entries']],['one','two'])
        self.policy['representatives']['apple (dried)']=dict(canonical_id='two',note='Invalid shadow.')
        with self.assertRaises(ValueError):self.run_review(first,second)

    def test_blocked_semantic_alias_and_retired_target_are_rejected(self):
        self.policy['entry_aliases']['one']=dict(label='heavy cream',aliases=['double cream'],note='Incorrect substitute.')
        result,_,decisions,_=self.run_review(self.entry('heavy cream'))
        self.assertNotIn('double cream',result['entries'][0]['aliases'])
        self.assertEqual(decisions[0]['status'],'hold_ambiguous')
        retired=self.entry('heavy cream');retired['status']='retired'
        with self.assertRaisesRegex(ValueError,'retired'):self.run_review(retired)

    def test_chili_dish_is_not_pluralized_as_peppers(self):
        self.policy['entry_rule_holds']['one']=dict(rules=['count-noun-number'],note='Prepared dish, not peppers.')
        result,*_=self.run_review(self.entry('chili (canned)'))
        names=result['entries'][0]['aliases']
        self.assertIn('canned chilli',names)
        self.assertNotIn('canned chilies',names)
        self.assertNotIn('canned chillies',names)

    def test_build_requires_matching_baseline_approval_and_output_pin(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            seed=dict(schema_version=1,dimension='ingredient',entries=[self.entry('egg')])
            (root/'seed.json').write_bytes(aliases.encoded(seed))
            (root/'sources.json').write_text('{"schema_version":1,"sources":[]}\n')
            self.policy['base_seed_checksum']=aliases.digest(root/'seed.json')
            self.policy['base_sources_checksum']=aliases.digest(root/'sources.json')
            def run(output,**updates):
                policy={**self.policy,**updates}
                (root/'policy.json').write_bytes(aliases.encoded(policy))
                args=argparse.Namespace(command='build',seed=root/'seed.json',sources=root/'sources.json',policy=root/'policy.json',output=root/output)
                aliases.run(args)
            checksum=self.run_review(self.entry('egg'))[3]['output_checksum']
            with self.assertRaisesRegex(ValueError,'not approved'):run('no',approved=False,expected_output_checksum=checksum)
            with self.assertRaisesRegex(ValueError,'not approved'):run('bad',approved=True,expected_output_checksum='0'*64)
            with self.assertRaisesRegex(ValueError,'Stale'):run('stale',approved=True,expected_output_checksum=checksum,base_seed_checksum='0'*64)
            run('one',approved=True,expected_output_checksum=checksum)
            run('two',approved=True,expected_output_checksum=checksum)
            self.assertEqual((root/'one/ingredients.json').read_bytes(),(root/'two/ingredients.json').read_bytes())
            with self.assertRaisesRegex(ValueError,'fresh'):run('one',approved=True,expected_output_checksum=checksum)


if __name__=='__main__':
    unittest.main()
