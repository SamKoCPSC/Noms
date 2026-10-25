"""Source-grounded follow-up preserves the complete published ingredient seed."""
import copy
import json
import unittest

import rename_labels
import fill_gaps as gaps


class FollowupTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy=json.loads(gaps.FOLLOWUP_POLICY.read_text())
        cls.seed=rename_labels.restore_baseline(json.loads((gaps.ROOT/'data/vocabulary/ingredients.json').read_text()))
        cls.sources=json.loads((gaps.ROOT/'data/vocabulary/sources.json').read_text())
        cls.base=gaps.restore_baseline(cls.seed,cls.policy)
        cls.bylabel={e['label']:e for e in cls.seed['entries']}

    def test_replay_and_all_previous_fields_are_preserved(self):
        result,summary=gaps.apply_review(self.base,self.sources,self.policy)
        self.assertEqual(result,self.seed)
        self.assertEqual(summary['output_checksum'],self.policy['expected_output_checksum'])
        old=json.loads(gaps.DEFAULT_POLICY.read_text())
        self.assertEqual(gaps.checksum(self.base),old['expected_output_checksum'])
        current={e['id']:e for e in self.seed['entries']}
        for entry in self.base['entries']:
            expected=copy.deepcopy(entry)
            expected['aliases']+=self.policy['alias_additions'].get(entry['id'],{}).get('aliases',[])
            self.assertEqual(current[entry['id']],expected)

    def test_preparation_and_biological_forms_stay_distinct(self):
        for names in [
            ['coriander seed','ground coriander seed','coriander leaf'],
            ['allspice','allspice (ground)'],
            ['daylily flower buds','dried daylily buds'],
            ['roselle calyx','dried roselle calyces','roselle leaves','sorrel leaf'],
            ['pectin','powdered pectin','pectin (liquid)'],
            ['vanilla extract','imitation vanilla flavouring'],
            ['golden raisins','seedless golden raisin (dried)'],
            ['pul biber','Urfa biber','Aleppo pepper'],
        ]:
            self.assertEqual(len({self.bylabel[n]['id'] for n in names}),len(names),names)

    def test_ambiguous_words_and_brands_are_not_unconditional_aliases(self):
        all_new={name.lower() for e in self.policy['additions'] for name in [e['label'],*e['aliases']]}
        for name in ['sorrel','dried sorrel','sorrel jam','lily buds','sour pork sausage','ground red pepper','vanilla essence','peppadew']:
            self.assertNotIn(name,all_new)
        for name in ['ground red pepper','almond essence','vanilla essence','polenta','light raisins','sweet peppadew peppers']:
            self.assertEqual(self.policy['reviewed_names'][name]['action'],'hold')
            self.assertIsNone(self.policy['reviewed_names'][name]['canonical_id'])

    def test_source_contexts_do_not_depend_on_provider_country(self):
        entries={e['id']:e['label'] for e in self.seed['entries']}
        contexts=self.policy['context_rows']
        sorrel=[d for key,d in contexts.items() if key.startswith('mealdb:53559:')]
        self.assertEqual({entries[d['canonical_ids'][0]] for d in sorrel},
                         {'dried roselle calyces','roselle jam'})
        for d in sorrel:self.assertIn('recipe',d['note'])

    def test_aliases_preserve_skin_and_bone_attributes(self):
        for name,label in [('boneless skinless chicken breasts','chicken breast (skinless, boneless)'),
                           ('boneless skinless chicken thighs','chicken thigh (skinless, boneless)')]:
            self.assertIn(name,self.bylabel[label]['aliases'])
        self.assertNotIn('chicken breasts',self.bylabel['chicken breast (skinless, boneless)']['aliases'])
        self.assertTrue(any(d['name']=='swedes' for d in self.policy['reserved_aliases']))
        for label,bare,qualified in [('black pepper','pepper','pepper (black)'),('red bell pepper','red pepper','red pepper (bell)')]:
            self.assertIn(qualified,self.bylabel[label]['aliases'])
            self.assertNotIn(bare,self.bylabel[label]['aliases'])

    def test_followup_rejects_prior_stage_or_stale_input(self):
        with self.assertRaisesRegex(ValueError,'Stale'):
            gaps.apply_review(gaps.restore_baseline(self.base,json.loads(gaps.DEFAULT_POLICY.read_text())),self.sources,self.policy)
        p=copy.deepcopy(self.policy);p['base_sources_checksum']='0'*64
        with self.assertRaisesRegex(ValueError,'Stale'):gaps.apply_review(self.base,self.sources,p)


if __name__=='__main__':unittest.main()
