"""Tests of the evidence gate itself. Test fixtures are NEVER acceptance evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image
import ux


class GateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.project = self.root / 'project'
        self.project.mkdir()
        self.catalog = {'fixture': {'id': 'unit-test-only'}, 'screens': [{'id': 'chat'}],
                        'journeys': [{'id': 'send', 'states': ['chat', 'chat'], 'operations': ['send_text']}]}
        self.api = {'operations': [{'id': 'send_text', 'status': 'existing'}]}
        self.save(self.project / 'flow-catalog.json', self.catalog)
        self.save(self.project / 'api-map.json', self.api)
        self.receipt = {'schema_version': 1, 'catalog_sha256': ux.sha(self.project / 'flow-catalog.json'),
                        'api_map_sha256': ux.sha(self.project / 'api-map.json'), 'source_sha256': 'test-source',
                        'fixture': 'unit-test-only', 'claim': 'generated-target', 'execution_mode': 'fixture',
                        'screens': [], 'journeys': []}
        # Deliberately tiny synthetic images; no connection to actual product acceptance.
        Image.new('RGB', (8, 12), 'white').save(self.root / 'reference.png')
        Image.new('RGB', (8, 12), 'gray').save(self.root / 'native.png')
        self.save(self.root / 'artifact.json', {'unit_test_fixture_only': True})
        for locale in ux.LOCALES:
            build = {'source_sha256': 'test-source', 'run_id': 'test-run-' + locale, 'binary_sha256': 'a' * 64}
            self.save(self.root / (locale + '-build.json'), build)
            inspect = {'source_sha256': 'test-source', 'native_sha256': ux.sha(self.root / 'native.png'),
                       'screen': 'chat', 'locale': locale, 'input_method': 'native', 'run_id': build['run_id'],
                       **{k: self.record('artifact.json') for k in ('snapshot', 'tree', 'layout', 'input_trace')},
                       'build_receipt': self.record(locale + '-build.json'),
                       'checks': {k: True for k in ('native_controls', 'no_screen_raster', 'no_clipping',
                                                   'text_legible', 'enabled_state', 'hit_targets')}}
            self.save(self.root / (locale + '-inspection.json'), inspect)
            ref = dict(self.record('reference.png'), kind='generated_target', locale=locale)
            self.receipt['screens'].append({'screen': 'chat', 'locale': locale, 'pixels': [8, 12],
                'reference': ref, 'native': self.record('native.png'),
                'inspection': self.record(locale + '-inspection.json'),
                'review': {'reviewer': 'Unit test only', 'notes': 'Synthetic gate fixture, not a UI review.',
                           'design_match': 9, 'verdict': 'pass', 'criteria': {k: True for k in ux.CRITERIA},
                           'reference_sha256': ref['sha256'], 'native_sha256': ux.sha(self.root / 'native.png')}})
            trace = {'source_sha256': 'test-source', 'journey': 'send', 'locale': locale,
                     'input_method': 'native', 'mock_backend': True, 'visited_states': ['chat', 'chat'],
                     'run_id': build['run_id'], 'build_receipt': self.record(locale + '-build.json'),
                     **{k: self.record('artifact.json') for k in ('input_trace', 'assertion_log', 'backend_log')},
                     'checks': {k: True for k in ('expected_outcome', 'back_state', 'error_paths', 'no_duplicate_effects')}}
            self.save(self.root / (locale + '-journey.json'), trace)
            self.receipt['journeys'].append({'journey': 'send', 'locale': locale,
                                              'evidence': self.record(locale + '-journey.json')})

    def save(self, path, value):
        path.write_text(json.dumps(value))

    def record(self, name):
        return {'path': name, 'sha256': ux.sha(self.root / name)}

    def evaluate(self):
        path = self.root / 'acceptance.json'
        self.save(path, self.receipt)
        return ux.gate(path, self.project, current_source='test-source')

    def mutate_artifact(self, key, updates):
        row = self.receipt['screens'][0] if key == 'inspection' else self.receipt['journeys'][0]
        record = row[key]
        path = self.root / record['path']
        value = ux.read(path)
        value.update(updates)
        self.save(path, value)
        record['sha256'] = ux.sha(path)

    def test_complete_fixture_evidence_passes_only_fixture_scope(self):
        result = self.evaluate()
        self.assertTrue(result['accepted'], result['errors'])
        self.assertEqual(result['score'], 9)
        self.assertFalse(result['production_ready'])

    def test_missing_evidence_fails_without_score(self):
        result = ux.gate(self.root / 'missing.json', self.project, 'test-source')
        self.assertFalse(result['accepted'])
        self.assertIsNone(result['score'])

    def test_empty_catalog_cannot_pass(self):
        self.catalog['screens'] = []
        self.catalog['journeys'] = []
        self.save(self.project / 'flow-catalog.json', self.catalog)
        self.receipt['catalog_sha256'] = ux.sha(self.project / 'flow-catalog.json')
        self.receipt['screens'] = []
        self.receipt['journeys'] = []
        self.assertFalse(self.evaluate()['accepted'])

    def test_missing_or_duplicate_locale_fails(self):
        self.receipt['screens'][1] = copy.deepcopy(self.receipt['screens'][0])
        self.assertFalse(self.evaluate()['accepted'])

    def test_omitted_journey_fails(self):
        self.receipt['journeys'].pop()
        self.assertFalse(self.evaluate()['accepted'])

    def test_mock_cannot_be_promoted_to_live(self):
        self.receipt['execution_mode'] = 'live'
        self.assertFalse(self.evaluate()['accepted'])

    def test_live_requires_all_capabilities_implemented(self):
        self.receipt['execution_mode'] = 'live'
        self.api['operations'][0]['status'] = 'missing'
        self.save(self.project / 'api-map.json', self.api)
        self.receipt['api_map_sha256'] = ux.sha(self.project / 'api-map.json')
        result = self.evaluate()
        self.assertFalse(result['accepted'])
        self.assertTrue(any('Unimplemented' in e for e in result['errors']))

    def test_fixture_can_cover_an_explicit_backend_gap(self):
        self.api['operations'][0]['status'] = 'missing'
        self.save(self.project / 'api-map.json', self.api)
        self.receipt['api_map_sha256'] = ux.sha(self.project / 'api-map.json')
        result = self.evaluate()
        self.assertTrue(result['accepted'], result['errors'])
        self.assertFalse(result['production_ready'])
        self.assertEqual(result['remaining_capabilities'], ['send_text'])

    def test_generated_target_is_not_actual_wechat(self):
        self.receipt['claim'] = 'wechat'
        self.assertFalse(self.evaluate()['accepted'])

    def test_actual_capture_needs_version_and_device(self):
        self.receipt['claim'] = 'wechat'
        for row in self.receipt['screens']:
            row['reference']['kind'] = 'wechat_capture'
        self.assertFalse(self.evaluate()['accepted'])

    def test_source_change_invalidates_review(self):
        self.receipt['source_sha256'] = 'different'
        self.assertFalse(self.evaluate()['accepted'])

    def test_catalog_change_invalidates_review(self):
        self.catalog['new_requirement'] = True
        self.save(self.project / 'flow-catalog.json', self.catalog)
        self.assertFalse(self.evaluate()['accepted'])

    def test_file_tampering_invalidates_evidence(self):
        (self.root / 'artifact.json').write_text('{}')
        self.assertFalse(self.evaluate()['accepted'])

    def test_reference_cannot_be_used_as_native(self):
        self.receipt['screens'][0]['native'] = self.record('reference.png')
        self.assertFalse(self.evaluate()['accepted'])

    def test_score_cannot_be_bool_nan_or_out_of_range(self):
        for value in (True, float('nan'), float('inf'), 8.99, 11, '9'):
            with self.subTest(value=value):
                self.receipt['screens'][0]['review']['design_match'] = value
                self.assertFalse(self.evaluate()['accepted'])

    def test_incomplete_visual_criteria_fails(self):
        del self.receipt['screens'][0]['review']['criteria']['typography']
        self.assertFalse(self.evaluate()['accepted'])

    def test_old_pair_review_fails(self):
        self.receipt['screens'][0]['review']['native_sha256'] = 'old'
        self.assertFalse(self.evaluate()['accepted'])

    def test_dimension_mismatch_fails(self):
        self.receipt['screens'][0]['pixels'] = [16, 24]
        self.assertFalse(self.evaluate()['accepted'])

    def test_reference_locale_mismatch_fails(self):
        self.receipt['screens'][0]['reference']['locale'] = 'cn'
        self.assertFalse(self.evaluate()['accepted'])

    def test_native_inspection_must_be_current(self):
        self.mutate_artifact('inspection', {'source_sha256': 'old-source'})
        self.assertFalse(self.evaluate()['accepted'])

    def test_native_controls_cannot_be_skipped(self):
        self.mutate_artifact('inspection', {'checks': {}})
        self.assertFalse(self.evaluate()['accepted'])

    def test_journey_must_use_native_input(self):
        self.mutate_artifact('evidence', {'input_method': 'direct_reducer_call'})
        self.assertFalse(self.evaluate()['accepted'])

    def test_journey_must_visit_all_states(self):
        self.mutate_artifact('evidence', {'visited_states': ['chat']})
        self.assertFalse(self.evaluate()['accepted'])

    def test_journey_needs_backend_effect_proof(self):
        self.mutate_artifact('evidence', {'backend_log': None})
        self.assertFalse(self.evaluate()['accepted'])

    def test_no_path_escape(self):
        with self.assertRaises(ValueError):
            ux.local(self.root, '../outside.json')
        with self.assertRaises(ValueError):
            ux.local(self.root, '/absolute.json')

    def test_output_never_overwrites_old_receipt(self):
        path = self.root / 'immutable.json'
        ux.write_new(path, {'first': True})
        with self.assertRaises(FileExistsError):
            ux.write_new(path, {'first': False})
        self.assertEqual(ux.read(path), {'first': True})


if __name__ == '__main__':
    unittest.main()
