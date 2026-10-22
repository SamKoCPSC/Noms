"""FoodOn adapter regressions: RDF meaning, canonical identity and reviewed output."""
import argparse
import copy
import json
from pathlib import Path
import tempfile
import unittest

import foodon


ONTOLOGY = '''<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
 xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
 xmlns:owl="http://www.w3.org/2002/07/owl#"
 xmlns:obo="http://purl.obolibrary.org/obo/"
 xmlns:oio="http://www.geneontology.org/formats/oboInOwl#">
 <owl:Ontology rdf:about="http://example.test/foodon">
  <owl:versionIRI rdf:resource="https://example.test/release/1"/>
  <owl:versionInfo>one</owl:versionInfo><owl:versionInfo>two</owl:versionInfo>
  <owl:imports rdf:resource="https://example.invalid/never-fetch.owl"/>
 </owl:Ontology>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00002403"><rdfs:label>food material</rdfs:label></owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000001">
  <rdfs:label xml:lang="en">garlic bulb</rdfs:label><rdfs:label xml:lang="fr">ail</rdfs:label>
  <rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/FOODON_00002403"/>
  <obo:IAO_0000115 xml:lang="en">An edible garlic bulb.</obo:IAO_0000115>
  <oio:hasExactSynonym>garlic</oio:hasExactSynonym><oio:hasExactSynonym> ＧＡＲＬＩＣ </oio:hasExactSynonym>
  <oio:hasExactSynonym xml:lang="fr">ail</oio:hasExactSynonym>
  <oio:hasBroadSynonym>allium food</oio:hasBroadSynonym>
  <oio:hasRelatedSynonym>garlic paste</oio:hasRelatedSynonym>
 </owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000002">
  <rdfs:label>garlic powder</rdfs:label><rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/FOODON_00002403"/>
 </owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000003"><rdfs:label>garlic plant</rdfs:label></owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000004">
  <rdfs:label>obsolete garlic paste</rdfs:label><owl:deprecated>true</owl:deprecated>
  <rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/FOODON_00002403"/>
 </owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000005">
  <rdfs:label>organism with food relation</rdfs:label>
  <rdfs:subClassOf><owl:Restriction><owl:someValuesFrom rdf:resource="http://purl.obolibrary.org/obo/FOODON_00002403"/></owl:Restriction></rdfs:subClassOf>
 </owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000006">
  <rdfs:label>intersected food</rdfs:label>
  <owl:equivalentClass><owl:Class><owl:intersectionOf rdf:parseType="Collection">
   <rdf:Description rdf:about="http://purl.obolibrary.org/obo/FOODON_00002403"/>
   <owl:Restriction><owl:someValuesFrom rdf:resource="http://purl.obolibrary.org/obo/FOODON_00000003"/></owl:Restriction>
  </owl:intersectionOf></owl:Class></owl:equivalentClass>
 </owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000007">
  <rdfs:label xml:lang="fr">seulement français</rdfs:label><rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/FOODON_00002403"/>
 </owl:Class>
 <owl:Class rdf:about="http://purl.obolibrary.org/obo/FOODON_00000008">
  <rdfs:label>123 - technical food (efsa foodex2)</rdfs:label><rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/FOODON_00002403"/>
 </owl:Class>
 <owl:Axiom><owl:annotatedSource rdf:resource="http://purl.obolibrary.org/obo/FOODON_00000001"/>
  <owl:annotatedProperty rdf:resource="http://www.geneontology.org/formats/oboInOwl#hasExactSynonym"/>
  <owl:annotatedTarget>garlic</owl:annotatedTarget><oio:hasDbXref>EXAMPLE:42</oio:hasDbXref>
 </owl:Axiom>
</rdf:RDF>'''
EXISTING = '11111111-1111-4111-8111-111111111111'
RETIRED = '22222222-2222-4222-8222-222222222222'
NEW = '33333333-3333-4333-8333-333333333333'


class AdapterTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.owl = self.root / 'foodon.owl'
        self.owl.write_text(ONTOLOGY)
        self.source = {'id': 'foodon', 'url': 'https://example.test/foodon.owl', 'version': 'one',
                       'checksum': foodon.digest(self.owl), 'license': 'CC-BY-4.0', 'attribution': 'Fixture'}
        self.descriptor = {'schema_version': 1, 'source': self.source, 'bytes': self.owl.stat().st_size,
                           'version_iri': 'https://example.test/release/1', 'ingredient_root': 'FOODON_00002403'}
        self.seed = {'schema_version': 1, 'dimension': 'ingredient', 'entries': [
            {'id': EXISTING, 'label': 'garlic', 'locale': 'en', 'aliases': ['fresh garlic'],
             'description': 'Original culinary description.', 'status': 'active', 'source_mappings': [],
             'provenance': {'origin': 'curated', 'references': []}},
            {'id': RETIRED, 'label': 'retired item', 'locale': 'en', 'aliases': [],
             'description': '', 'status': 'retired', 'source_mappings': [],
             'provenance': {'origin': 'curated', 'references': []}}]}
        self.manifest = {'schema_version': 1, 'sources': []}
        for name, value in [('descriptor', self.descriptor), ('seed', self.seed), ('sources', self.manifest),
                            ('pilot', [{'status': 'E', 'source_id': 'FOODON_00000001', 'ingredient': 'garlic',
                                        'reason': 'A usable discovery match.'},
                                       {'status': 'R', 'source_id': 'FOODON_00000003', 'ingredient': 'garlic', 'reason': 'Plant.'}])]:
            (self.root / (name + '.json')).write_text(json.dumps(value))
        self.args = argparse.Namespace(owl=self.owl, descriptor=self.root / 'descriptor.json',
            seed=self.root / 'seed.json', sources=self.root / 'sources.json', pilot=self.root / 'pilot.json',
            output=self.root / 'extracted')

    def draft(self):
        foodon.extract(self.args)
        review = foodon.load(self.args.output / 'review.json')
        self.args.review = self.root / 'approved.json'
        self.args.output = self.root / 'built'
        return review

    def build(self, review):
        self.args.review.write_text(json.dumps(review))
        return foodon.build(self.args)

    def test_extract_preserves_annotations_without_promoting_synonym_scopes(self):
        terms, metadata = foodon.read_ontology(self.owl, self.descriptor)
        garlic = terms['FOODON_00000001']
        self.assertEqual(garlic['canonical_proposal'], {'label': 'garlic bulb', 'aliases': ['garlic'],
                                                       'description': 'An edible garlic bulb.'})
        self.assertEqual(garlic['synonyms']['related'], ['garlic paste'])
        self.assertTrue(any(a['language'] == 'fr' for a in garlic['annotations']))
        self.assertIn('EXAMPLE:42', json.dumps(garlic['axiom_annotations']))
        self.assertEqual(len([m for m in metadata if m['property'] == foodon.OWL + 'versionInfo']), 2)
        self.assertIn('https://example.invalid/never-fetch.owl', json.dumps(metadata))

    def test_named_hierarchy_not_restriction_fillers_controls_eligibility(self):
        terms, _ = foodon.read_ontology(self.owl, self.descriptor)
        for suffix in ['03', '04', '05', '07']:
            self.assertFalse(terms['FOODON_000000' + suffix]['eligible'])
        self.assertTrue(terms['FOODON_00000006']['eligible'])
        self.assertNotIn('FOODON_00000003', terms['FOODON_00000006']['parents'])
        self.assertIn('technical_label', terms['FOODON_00000008']['flags'])

    def test_split_rdf_subjects_keep_forward_annotations(self):
        extra = '''<rdf:Description rdf:about="http://purl.obolibrary.org/obo/FOODON_00000002">
         <obo:IAO_0000115>A powder definition declared before the class.</obo:IAO_0000115>
        </rdf:Description>'''
        self.owl.write_text(ONTOLOGY.replace(' <owl:Ontology', extra + ' <owl:Ontology', 1))
        self.descriptor['source']['checksum'] = foodon.digest(self.owl)
        self.descriptor['bytes'] = self.owl.stat().st_size
        terms, _ = foodon.read_ontology(self.owl, self.descriptor)
        self.assertEqual(terms['FOODON_00000002']['canonical_proposal']['description'],
                         'A powder definition declared before the class.')

    def test_scoped_material_exceptions_are_explicit_and_cannot_revive_obsolete_terms(self):
        # FoodOn's actual annatto/teff terms use imported Plant Ontology parents.
        text = ONTOLOGY.replace('<rdfs:label>garlic plant</rdfs:label>',
            '<rdfs:label>garlic plant</rdfs:label><rdfs:subClassOf rdf:resource="http://purl.obolibrary.org/obo/PO_0009010"/>')
        self.owl.write_text(text)
        self.descriptor['source']['checksum'] = foodon.digest(self.owl)
        self.descriptor['bytes'] = self.owl.stat().st_size
        self.descriptor['reviewed_ingredient_terms'] = {
            'FOODON_00000003': 'Reviewed material exception fixture, not a general organism rule.',
            'FOODON_00000004': 'Even an explicit exception cannot revive a deprecated class.'}
        terms, _ = foodon.read_ontology(self.owl, self.descriptor)
        self.assertTrue(terms['FOODON_00000003']['eligible'])
        self.assertEqual(terms['FOODON_00000003']['parent_iris'], [foodon.OBO + 'PO_0009010'])
        self.assertEqual(terms['FOODON_00000003']['parents'], [])
        self.assertIn('outside_food_material_hierarchy', terms['FOODON_00000003']['flags'])
        self.assertIn('reviewed_ingredient_scope_exception', terms['FOODON_00000003']['flags'])
        self.assertFalse(terms['FOODON_00000004']['eligible'])
        self.descriptor['reviewed_ingredient_terms']['FOODON_00002403'] = 'No root import'
        with self.assertRaisesRegex(ValueError, 'Invalid reviewed ingredient scope exceptions'):
            foodon.read_ontology(self.owl, self.descriptor)

    def test_descriptor_changes_invalidate_approved_review(self):
        review = self.draft()
        review['decisions'][0].update(approved=True, relationship='exact')
        self.descriptor['reviewed_ingredient_terms'] = {'FOODON_00000003': 'New scope decision'}
        self.args.descriptor.write_text(json.dumps(self.descriptor))
        with self.assertRaisesRegex(ValueError, 'descriptor changed'):
            self.build(review)

    def test_corruption_wrong_release_and_xml_entities_fail_closed(self):
        corrupt = copy.deepcopy(self.descriptor)
        corrupt['source']['checksum'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'SHA-256'):
            foodon.read_ontology(self.owl, corrupt)
        corrupt = copy.deepcopy(self.descriptor)
        corrupt['version_iri'] = 'https://example.test/wrong'
        with self.assertRaisesRegex(ValueError, 'version IRI'):
            foodon.read_ontology(self.owl, corrupt)
        self.owl.write_text('<!DOCTYPE rdf:RDF [<!ENTITY example "no">]>' + ONTOLOGY)
        corrupt['source']['checksum'] = foodon.digest(self.owl)
        corrupt['bytes'] = self.owl.stat().st_size
        with self.assertRaisesRegex(ValueError, 'DTD/entities'):
            foodon.read_ontology(self.owl, corrupt)

    def test_coverage_match_is_pending_and_does_not_imply_exact_mapping(self):
        review = self.draft()
        self.assertEqual(len(review['decisions']), 1)
        self.assertIsNone(review['decisions'][0]['relationship'])
        with self.assertRaisesRegex(ValueError, 'No approved'):
            self.build(review)
        review['decisions'][0]['approved'] = True
        with self.assertRaisesRegex(ValueError, 'explicit reviewed relationship'):
            self.build(review)
        self.assertFalse(self.args.output.exists())

    def test_existing_identity_wording_and_retired_entries_survive_mapping(self):
        review = self.draft()
        review['decisions'][0].update(approved=True, relationship='narrower')
        self.build(review)
        result = foodon.load(self.args.output / 'ingredients.json')
        for field in ['id', 'label', 'aliases', 'description', 'locale', 'status']:
            self.assertEqual(result['entries'][0][field], self.seed['entries'][0][field])
        self.assertEqual(result['entries'][0]['provenance']['origin'], 'curated')
        self.assertEqual(result['entries'][1], self.seed['entries'][1])
        self.assertEqual(foodon.load(self.args.seed), self.seed)
        self.assertEqual(result['entries'][0]['source_mappings'][0]['relationship'], 'narrower')
        self.assertEqual(foodon.load(self.args.output / 'sources.json')['sources'], [self.source])
        self.assertIn('EXAMPLE:42', json.dumps(foodon.load(self.args.output / 'report.json')['selected_source_terms']))

    def test_import_uses_source_wording_and_persisted_identity_deterministically(self):
        review = self.draft()
        review['decisions'].append({'approved': True, 'action': 'import', 'term_id': 'FOODON_00000001',
                                   'canonical_id': NEW, 'relationship': 'exact'})
        self.build(review)
        first = (self.args.output / 'ingredients.json').read_bytes()
        entry = foodon.load(self.args.output / 'ingredients.json')['entries'][-1]
        self.assertEqual(entry['id'], NEW)
        self.assertEqual(entry['label'], 'garlic bulb')
        self.assertEqual(entry['aliases'], ['garlic'])
        self.assertEqual(entry['provenance'], {'origin': 'imported', 'references': ['foodon']})
        self.args.output = self.root / 'built-again'
        self.build(review)
        self.assertEqual((self.args.output / 'ingredients.json').read_bytes(), first)

    def test_alias_collisions_remain_distinct_and_are_reported(self):
        review = self.draft()
        review['decisions'] = [{'approved': True, 'action': 'import', 'term_id': 'FOODON_00000002',
            'canonical_id': NEW, 'relationship': 'exact', 'canonical': {'aliases': ['garlic']}}]
        report = self.build(review)
        self.assertEqual(report['entries'], 3)
        self.assertEqual(report['alias_collisions'], 1)

    def test_previous_import_can_be_retained_retired_with_its_uuid(self):
        review = self.draft()
        review['decisions'] = [{'approved': True, 'action': 'import', 'term_id': 'FOODON_00000002',
            'canonical_id': NEW, 'relationship': 'exact', 'canonical': {'status': 'retired'},
            'note': 'Retain a previously staged identity after editorial scope review.'}]
        self.build(review)
        entry = foodon.load(self.args.output / 'ingredients.json')['entries'][-1]
        self.assertEqual((entry['id'], entry['status']), (NEW, 'retired'))
        self.assertEqual(entry['source_mappings'][0]['external_id'], 'FOODON_00000002')
        self.args.output = self.root / 'bad-status'
        review['decisions'][0]['canonical']['status'] = 'deleted'
        with self.assertRaisesRegex(ValueError, 'Invalid canonical status'):
            self.build(review)

    def test_snapshot_base_changes_and_non_boolean_approval_are_rejected(self):
        review = self.draft()
        review['decisions'][0].update(approved='true', relationship='exact')
        with self.assertRaisesRegex(ValueError, 'JSON boolean'):
            self.build(review)
        review['decisions'][0]['approved'] = True
        review['source_checksum'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'different source snapshot'):
            self.build(review)
        review['source_checksum'] = self.source['checksum']
        self.args.seed.write_text(json.dumps(self.seed) + '\n')
        with self.assertRaisesRegex(ValueError, 'Base inputs changed'):
            self.build(review)

    def test_no_output_overwrite_or_new_identity_reuse(self):
        review = self.draft()
        review['decisions'][0].update(approved=True, relationship='exact')
        self.build(review)
        with self.assertRaisesRegex(ValueError, 'already exists'):
            self.build(review)
        self.args.output = self.root / 'invalid'
        review['decisions'][0]['action'] = 'import'
        with self.assertRaisesRegex(ValueError, 'new, persisted UUIDv4'):
            self.build(review)

    def test_no_exact_merge_and_no_retired_or_organism_import(self):
        review = self.draft()
        review['decisions'][0].update(approved=True, relationship='exact')
        review['decisions'].append({'approved': True, 'action': 'import', 'canonical_id': NEW,
                                   'term_id': 'FOODON_00000001', 'relationship': 'exact'})
        with self.assertRaisesRegex(ValueError, 'distinct canonical IDs'):
            self.build(review)
        for suffix in ['03', '04', '05']:
            review['decisions'] = [{'approved': True, 'action': 'import', 'canonical_id': NEW,
                                   'term_id': 'FOODON_000000' + suffix, 'relationship': 'exact'}]
            with self.assertRaisesRegex(ValueError, 'Not an eligible'):
                self.build(review)

    def test_invalid_canonical_wording_and_conflicting_source_metadata_fail(self):
        review = self.draft()
        review['decisions'] = [{'approved': True, 'action': 'import', 'canonical_id': NEW,
            'term_id': 'FOODON_00000002', 'relationship': 'exact', 'canonical': {'aliases': [' GARLIC POWDER ']}}]
        with self.assertRaisesRegex(ValueError, 'Repeated label/alias'):
            self.build(review)
        source = dict(self.source, checksum='0' * 64)
        self.args.sources.write_text(json.dumps({'schema_version': 1, 'sources': [source]}))
        review['base_sources_checksum'] = foodon.digest(self.args.sources)
        with self.assertRaisesRegex(ValueError, 'different metadata'):
            self.build(review)


if __name__ == '__main__':
    unittest.main()
