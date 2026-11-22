"""Exercise production process ordering and credential boundaries without a DB."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
STUB = '''#!/usr/bin/env python3
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
event = dict(name=name, args=sys.argv[1:],
             deployment_database=os.environ.get('DATABASE_URL') == os.environ['NOMS_TEST_DEPLOYMENT_URL'],
             runtime_database=os.environ.get('DATABASE_URL') == os.environ['NOMS_TEST_RUNTIME_URL'],
             migration_variable='MIGRATION_DATABASE_URL' in os.environ,
             prepared=os.environ.get('NOMS_MIGRATIONS_APPLIED'))
with open(os.environ['NOMS_TEST_EVENTS'], 'a') as f:
    f.write(json.dumps(event) + '\\n')
sys.exit(19 if name == os.environ.get('NOMS_TEST_FAIL_STAGE') else 0)
'''


class DeploymentSeeding(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        app = self.root / 'app'
        app.mkdir()
        self.entrypoint = app / 'entrypoint.sh'
        self.entrypoint.write_text((ROOT / 'entrypoint.sh').read_text().replace('/usr/local/app', str(app)))
        for name in ['psql', 'pgschema', 'seed-vocabulary', 'server']:
            f = app / name
            f.write_text(STUB)
            f.chmod(0o755)
        self.events_file = self.root / 'events.jsonl'
        runtime = 'postgresql://runtime:runtime-test@db:5432/noms'
        migration = 'postgresql://deployment:deployment-test@db:5432/noms'
        self.env = dict(os.environ, PATH=f'{app}:{os.environ["PATH"]}',
                        DATABASE_URL=runtime, MIGRATION_DATABASE_URL=migration,
                        NOMS_TEST_RUNTIME_URL=runtime, NOMS_TEST_DEPLOYMENT_URL=migration,
                        NOMS_TEST_EVENTS=str(self.events_file))

    def run_entrypoint(self, *args):
        result = subprocess.run(['bash', str(self.entrypoint), *args], env=self.env,
                                text=True, capture_output=True)
        events = [json.loads(x) for x in self.events_file.read_text().splitlines()] if self.events_file.exists() else []
        return result, events

    def test_startup_prepares_before_server_and_limits_seed_credentials(self):
        result, events = self.run_entrypoint()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([x['name'] for x in events], ['psql', 'pgschema', 'psql', 'seed-vocabulary', 'server'])
        seed, server = events[-2:]
        self.assertTrue(seed['deployment_database'])
        self.assertIn('--if-changed', seed['args'])
        self.assertIn('--file', seed['args'])
        self.assertIn('--sources', seed['args'])
        self.assertTrue(server['runtime_database'])
        self.assertFalse(server['migration_variable'])
        self.assertEqual(server['prepared'], '1')
        for event in [events[0], events[2]]:
            self.assertIn('--set=ON_ERROR_STOP=1', event['args'])

    def test_predeploy_prepares_once_and_exits_without_starting_server(self):
        result, events = self.run_entrypoint('--prepare-database')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([x['name'] for x in events], ['psql', 'pgschema', 'psql', 'seed-vocabulary'])

    def test_failure_in_extensions_schema_or_seed_blocks_deployment_and_server(self):
        for stage in ['psql', 'pgschema', 'seed-vocabulary']:
            with self.subTest(stage=stage):
                self.events_file.unlink(missing_ok=True)
                self.env['NOMS_TEST_FAIL_STAGE'] = stage
                result, events = self.run_entrypoint()
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(events[-1]['name'], stage)
                self.assertNotIn('server', [x['name'] for x in events])

    def test_unknown_command_fails_before_any_database_work(self):
        result, events = self.run_entrypoint('--typo')
        self.assertEqual(result.returncode, 2)
        self.assertEqual(events, [])

    def test_same_role_cannot_prepare_or_seed(self):
        self.env['MIGRATION_DATABASE_URL'] = self.env['DATABASE_URL']
        result, events = self.run_entrypoint()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(events, [])

    def test_both_railway_configs_use_the_one_shot_preparation_command(self):
        import tomllib
        toml = tomllib.loads((ROOT / 'railway.toml').read_text())
        config = json.loads((ROOT / 'railway.json').read_text())
        expected = ['/usr/local/app/entrypoint.sh --prepare-database']
        self.assertEqual(config['deploy']['preDeployCommand'], expected)
        self.assertEqual(toml['deploy']['preDeployCommand'], expected)


if __name__ == '__main__':
    unittest.main()
