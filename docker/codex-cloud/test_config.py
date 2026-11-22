"""Regression checks for isolation/trust contracts; no daemon or network needed."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location('cloud_prepare', HERE / 'prepare.py')
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class CloudConfigTests(unittest.TestCase):
    def test_every_network_stage_keeps_trust_and_upstream_versions(self):
        for source in (ROOT / 'docker/development/Dockerfile', ROOT / 'docker/postgres/Dockerfile'):
            original = source.read_text()
            generated = prepare.dockerfile(original)
            self.assertEqual(original.count('\nFROM '), generated.count('\nFROM '))
            for line in generated.splitlines():
                if line.startswith('RUN '):
                    self.assertIn('id=cloud_ca,required=true,mode=0444', line)
            self.assertNotIn('apt-get update', generated)
            self.assertNotIn('--insecure', generated)
            self.assertNotIn('GOSUMDB=off', generated)
            for line in original.splitlines():
                if line.startswith('FROM '):
                    self.assertIn(line, generated)
        pg = prepare.dockerfile((ROOT / 'docker/postgres/Dockerfile').read_text())
        self.assertIn('chmod 755 /docker-entrypoint-initdb.d/init-cron-config.sh', pg)
        self.assertIn('sha256sum --check', pg)

    def test_lifecycle_preserves_ownership_and_avoids_build_recreation(self):
        original = (ROOT / 'scripts/dev-environment.sh').read_text()
        generated = prepare.lifecycle(original, ROOT, Path('/tmp/cloud-state'))
        self.assertIn('create --no-build', generated)
        self.assertNotIn('create --build', generated)
        self.assertIn(original[original.index('validate_project_ownership() {'):original.index('compose_command() {')], generated)
        marker = generated.index('Starting isolated services for $instance_id')
        self.assertLess(generated.index('rm --force app schema-init minio minio-init mock-oauth'), marker)
        with self.assertRaises(ValueError):
            prepare.lifecycle(original.replace('create --build', 'create --changed-upstream'), ROOT, Path('/tmp/cloud-state'))

    def test_proxy_refresh_preserves_registry_auth_and_named_proxy_entries(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'config.json'
            original = {'auths': {'registry.example.test': {'auth': 'test-only'}},
                        'credsStore': 'example-helper', 'proxies': {'default': {'httpProxy': 'http://old.invalid:8080'},
                                                                  'other-daemon': {'httpProxy': 'http://keep.invalid:8080'}}}
            path.write_text(json.dumps(original))
            with patch.dict(os.environ, {'HTTP_PROXY': 'http://proxy:8080', 'HTTPS_PROXY': 'http://proxy:8080', 'NO_PROXY': 'example.test'}, clear=True), patch('socket.gethostbyname', return_value='192.0.2.10'):
                prepare.refresh_proxy(path)
            result = json.loads(path.read_text())
            self.assertEqual(result['auths'], original['auths'])
            self.assertEqual(result['credsStore'], original['credsStore'])
            self.assertEqual(result['proxies']['other-daemon'], original['proxies']['other-daemon'])
            self.assertEqual(result['proxies']['default']['httpProxy'], 'http://192.0.2.10:8080')
            self.assertIn('mock-oauth', result['proxies']['default']['noProxy'])
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)

    def test_generated_compose_merges_real_lifecycle_services(self):
        if not shutil.which('docker'):
            self.skipTest('Docker Compose client is required for configuration rendering')
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            subprocess.run(['python3', str(HERE / 'prepare.py'), '--state', str(state)], check=True, stdout=subprocess.DEVNULL)
            (state / 'instance.env').write_text('NOMS_PORT_BOOTSTRAP=1\n')
            env = os.environ.copy()
            env.update(NOMS_INSTANCE_ID='noms-cloud-config-test', NOMS_INSTANCE_TOKEN='configuration-only',
                       NOMS_WORKTREE=str(ROOT), NOMS_RUNTIME_DIR=str(state), NOMS_CLOUD_STATE_DIR=str(state),
                       NOMS_CLOUD_CA='/etc/ssl/certs/ca-certificates.crt')
            result = json.loads(subprocess.check_output(['docker', 'compose', '--file', str(ROOT / 'docker-compose.yml'),
                                                         '--file', str(state / 'compose.override.yml'), 'config', '--format', 'json'], env=env))
            services = result['services']
            self.assertEqual(services['schema-init']['image'], 'noms-codex-tools:noms-cloud-config-test')
            self.assertEqual(services['postgres']['image'], 'noms-codex-postgres:noms-cloud-config-test')
            self.assertEqual(services['app']['labels']['com.noms.dev-token'], 'configuration-only')
            mounts = {volume['target']: volume for volume in services['app']['volumes']}
            self.assertTrue(mounts['/run/cloud-ca.pem']['read_only'])
            self.assertEqual(mounts['/workspace/assets/tailwind.css']['source'], str(state / 'tailwind.css'))
            self.assertEqual(mounts['/workspace']['source'], str(ROOT))
            self.assertIn('schema-init', services['app']['depends_on'])
            subprocess.run(['bash', '-n', str(state / 'dev-environment.sh')], check=True)


if __name__ == '__main__':
    unittest.main()
