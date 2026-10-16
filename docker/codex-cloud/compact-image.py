#!/usr/bin/env python3
"""Flatten the local development image while retaining its complete runtime config."""
import hashlib, io, json, os, subprocess, tarfile, tempfile, sys
from pathlib import Path

def run(*args):
    return subprocess.check_output(['docker', *args])
if len(sys.argv) != 3:
    raise SystemExit('Usage: compact-image.py SOURCE_IMAGE TARGET_IMAGE')
source, target = sys.argv[1:3]
original = json.loads(run('image', 'inspect', source))[0]

container = run('create', source).decode().strip()
try:
    for volume_path in original['Config'].get('Volumes') or {}:
        contents = run('cp', container + ':' + volume_path + '/.', '-')
        with tarfile.open(fileobj=io.BytesIO(contents)) as volume_tar:
            members = volume_tar.getmembers()
            # docker cp emits one directory entry for an empty volume root.
            # Even nested empty directories count as contents we must not omit.
            if len(members) > 1 or any(not member.isdir() for member in members):
                raise RuntimeError('Export would omit volume data; refusing compaction')
    with tempfile.TemporaryDirectory(prefix='noms-compact-', dir=os.environ.get('NOMS_CLOUD_STATE_DIR')) as directory:
        layer = Path(directory) / 'layer.tar'
        subprocess.run(['docker', 'export', '--output', str(layer), container], check=True)
        subprocess.run(['docker', 'rm', '--volumes', container], check=True, stdout=subprocess.DEVNULL)
        container = None
        digest = hashlib.file_digest(layer.open('rb'), 'sha256').hexdigest()
        platform = run('info', '--format', '{{.OSType}}/{{.Architecture}}').decode().strip().split('/')
        architecture = original.get('Architecture') or {'x86_64': 'amd64', 'aarch64': 'arm64'}.get(platform[1], platform[1])
        config = {'architecture': architecture, 'os': original.get('Os') or platform[0]}
        if original.get('Variant'):
            config['variant'] = original['Variant']
        config.update(config=original['Config'], rootfs={'type': 'layers', 'diff_ids': ['sha256:' + digest]}, history=[{'created_by': 'cloud development image compaction'}])
        data = json.dumps(config).encode()
        config_name = hashlib.sha256(data).hexdigest() + '.json'
        manifest = json.dumps([{'Config': config_name, 'RepoTags': [target], 'Layers': ['layer.tar']}]).encode()
        loader = subprocess.Popen(['docker', 'load'], stdin=subprocess.PIPE)
        with tarfile.open(fileobj=loader.stdin, mode='w|') as tar:
            for name, blob in [(config_name, data), ('manifest.json', manifest)]:
                info = tarfile.TarInfo(name); info.size = len(blob)
                tar.addfile(info, io.BytesIO(blob))
            tar.add(layer, arcname='layer.tar')
        loader.stdin.close()
        if loader.wait() != 0:
            raise RuntimeError('docker load failed')
    compact = json.loads(run('image', 'inspect', target))[0]
    if compact['Architecture'] != architecture or compact['Os'] != config['os'] or compact['Config'] != original['Config']:
        raise RuntimeError('Compaction changed runtime configuration or platform; retain the original image')
    print('Compaction passed: complete runtime configuration preserved')
finally:
    if container:
        subprocess.run(['docker', 'rm', '--volumes', container], check=True, stdout=subprocess.DEVNULL)
