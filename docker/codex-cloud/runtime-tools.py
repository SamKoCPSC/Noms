#!/usr/bin/env python3
"""Install pinned Dioxus helpers using verified TLS and the inherited proxy."""
import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile


def download(url):
    return subprocess.check_output(['curl', '--fail', '--silent', '--show-error', '--location',
                                    '--cacert', os.environ['NOMS_CLOUD_CA'], url])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('state', type=Path)
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.machine() not in ('x86_64', 'amd64'):
        parser.error('Currently validated for Linux amd64 only; review helper artifacts for other architectures')
    tools = args.state / 'runtime-tools'
    tools.mkdir(parents=True, exist_ok=True)
    # Immutable expected binary digests observed over verified TLS during onboarding.
    expected = {'esbuild': 'fd49340cbb5f7ca3ce6f313db78d056de21f07338135ec8dfad63294a304fd2c',
                'tailwindcss': '9d258a7786c22f8572aea20ed35848f8cbef1d06277e4e6b97ad8561ffc21e07'}
    for name, digest in expected.items():
        target = tools / name
        if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest() == digest:
            continue
        if name == 'esbuild':
            metadata = json.loads(download('https://registry.npmjs.org/@esbuild/linux-x64/0.27.3'))
            data = download('https://registry.npmjs.org/@esbuild/linux-x64/-/linux-x64-0.27.3.tgz')
            integrity = 'sha512-' + base64.b64encode(hashlib.sha512(data).digest()).decode()
            if integrity != metadata['dist']['integrity']:
                raise RuntimeError('esbuild npm integrity mismatch')
            with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
                binary = archive.extractfile('package/bin/esbuild').read()
        else:
            binary = download('https://github.com/tailwindlabs/tailwindcss/releases/download/v4.1.5/tailwindcss-linux-x64')
        if hashlib.sha256(binary).hexdigest() != digest:
            raise RuntimeError(name + ' checksum mismatch; review upstream artifact before updating the pin')
        target.write_bytes(binary)
        target.chmod(0o755)
    wasm = tools / 'wasm-bindgen'
    if not wasm.exists() or subprocess.check_output([str(wasm), '--version'], text=True).strip() != 'wasm-bindgen 0.2.121':
        # Keep Cargo/checksum verification enabled; do not use Dioxus's webpki-only downloader.
        # Host Cargo is required so the large development image need not be cloned again.
        with tempfile.TemporaryDirectory(prefix='wasm-build-', dir=args.state) as temporary:
            env = os.environ.copy()
            env.update(CARGO_HTTP_CAINFO=os.environ['NOMS_CLOUD_CA'], SSL_CERT_FILE=os.environ['NOMS_CLOUD_CA'],
                       CARGO_BUILD_JOBS=os.environ.get('CARGO_BUILD_JOBS', '2'), CARGO_TARGET_DIR=temporary + '/target')
            subprocess.run(['cargo', 'install', 'wasm-bindgen-cli', '--version', '0.2.121', '--locked',
                            '--root', temporary + '/install'], env=env, check=True)
            wasm.write_bytes((Path(temporary) / 'install/bin/wasm-bindgen').read_bytes())
            wasm.chmod(0o755)
    (args.state / 'runtime-tools.sha256').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.name + '\n' for p in sorted(tools.iterdir()) if p.is_file()))
    print('Pinned runtime tools ready')


if __name__ == '__main__':
    main()
