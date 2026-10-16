#!/usr/bin/env python3
"""Generate cloud-only adaptations without editing the shared development files."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import tomllib
from urllib.parse import urlsplit, urlunsplit

ROOT = Path(__file__).resolve().parents[2]
CA_SETTINGS = ('export SSL_CERT_FILE=/run/secrets/cloud_ca '
               'CARGO_HTTP_CAINFO=/run/secrets/cloud_ca '
               'CURL_CA_BUNDLE=/run/secrets/cloud_ca '
               'GIT_SSL_CAINFO=/run/secrets/cloud_ca && ')


def dockerfile(source):
    """Add trust to every RUN, including every stage; retain upstream pins."""
    blocks = re.split(r'(?m)(?=^[A-Z]+ )', source)
    result = []
    for block in blocks:
        if block.startswith('RUN '):
            block = block.replace('apt-get ', 'apt-get -o Acquire::https::CaInfo=/run/secrets/cloud_ca ')
            block = block.replace('chmod +x /docker-entrypoint-initdb.d/init-cron-config.sh',
                                  'chmod 755 /docker-entrypoint-initdb.d/init-cron-config.sh')
            block = block.replace('&& cp /usr/local/cargo/bin/dx /usr/local/bin/dx',
                                  '&& cp /usr/local/cargo/bin/dx /usr/local/bin/dx && rm -rf /usr/local/cargo/registry /usr/local/cargo/git /tmp/cargo-install*')
            # Keep cached source/compile trees out of vfs image layers.
            if 'go install -p 2 ' in block:
                first, *rest = block.splitlines()
                block = first + ' && rm -rf /go/pkg /root/.cache/go-build\n' + '\n'.join(rest) + '\n'
            block = 'RUN --mount=type=secret,id=cloud_ca,required=true,mode=0444 ' + CA_SETTINGS + block[4:]
        result.append(block)
    return ''.join(result)


def lifecycle(source, root, state):
    def replace_once(old, new):
        nonlocal source
        if source.count(old) != 1:
            raise ValueError('Development lifecycle changed; review cloud adaptation: ' + old)
        source = source.replace(old, new, 1)
    # Shell strings are emitted with single-quote escaping, never interpolated as code.
    import shlex
    line = next(line for line in source.splitlines() if line.startswith('readonly REPOSITORY_ROOT='))
    replace_once(line, 'readonly REPOSITORY_ROOT=' + shlex.quote(str(root)))
    original = '            --file "$worktree/docker-compose.yml" \\\n'
    replace_once(original, original + '            --file ' + shlex.quote(str(state / 'compose.override.yml')) + ' \\\n')
    replace_once('create --build', 'create --no-build')
    marker = '    echo "Starting isolated services for $instance_id..."'
    replace_once(marker, '''    # Release bootstrap vfs roots before final Compose recreation.
    validate_project_ownership "$instance_id" "$instance_token"
    compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" stop app schema-init minio minio-init mock-oauth >/dev/null
    compose_command "$instance_id" "$runtime_dir" "$worktree" "$instance_token" rm --force app schema-init minio minio-init mock-oauth >/dev/null
''' + marker)
    return source


def refresh_proxy(config_path):
    config = json.loads(config_path.read_text()) if config_path.exists() else {}
    defaults = config.setdefault('proxies', {}).setdefault('default', {})
    for name, key in [('HTTP_PROXY', 'httpProxy'), ('HTTPS_PROXY', 'httpsProxy'), ('NO_PROXY', 'noProxy')]:
        value = os.environ.get(name, os.environ.get(name.lower()))
        if value is not None:
            if key != 'noProxy':
                url = urlsplit(value)
                if url.hostname == 'proxy':
                    credentials = url.netloc.rpartition('@')[0] + '@' if '@' in url.netloc else ''
                    value = urlunsplit((url.scheme, credentials + socket.gethostbyname('proxy') + ':' + str(url.port or 8080), url.path, url.query, url.fragment))
            defaults[key] = value
    local = ['localhost', '127.0.0.1', '::1', 'postgres', 'minio', 'mock-oauth', 'mock-github-api']
    defaults['noProxy'] = ','.join(dict.fromkeys(filter(None, defaults.get('noProxy', '').split(',') + local)))
    config_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = config_path.with_suffix('.cloud-tmp')
    temporary.touch(mode=0o600, exist_ok=False)
    try:
        temporary.write_text(json.dumps(config, indent=2) + '\n')
        temporary.replace(config_path)
    finally:
        temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    default = Path(os.environ.get('TMPDIR', '/tmp')) / 'noms-codex-cloud' / hashlib.sha256(str(ROOT).encode()).hexdigest()[:12]
    parser.add_argument('--state', type=Path, default=Path(os.environ.get('NOMS_CLOUD_STATE_DIR', str(default))))
    parser.add_argument('--refresh-proxy', action='store_true')
    args = parser.parse_args()
    development = (ROOT / "docker/development/Dockerfile").read_text()
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    wasm_versions = {p["version"] for p in packages if p["name"] == "wasm-bindgen"}
    if "cargo install dioxus-cli@0.7.9 --locked" not in development or wasm_versions != {"0.2.121"}:
        parser.error("Dioxus/wasm-bindgen versions changed; review runtime helper pins before cloud startup")
    state = args.state.resolve()
    if state == ROOT or ROOT in state.parents:
        parser.error('Cloud state must be outside the repository checkout')
    state.mkdir(parents=True, exist_ok=True, mode=0o700)
    for name, source in [('Development', ROOT / 'docker/development/Dockerfile'), ('Postgres', ROOT / 'docker/postgres/Dockerfile')]:
        (state / (name + '.Dockerfile')).write_text(dockerfile(source.read_text()))
    (state / 'dev-environment.sh').write_text(lifecycle((ROOT / 'scripts/dev-environment.sh').read_text(), ROOT, state))
    (state / 'compose.override.yml').write_text((ROOT / 'docker/codex-cloud/compose.override.yml').read_text())
    css = state / 'tailwind.css'
    if not css.exists():
        css.write_bytes((ROOT / 'assets/tailwind.css').read_bytes())
    base = (ROOT / 'docker/postgres/Dockerfile').read_text().splitlines()[0].removeprefix('FROM ')
    if '@sha256:' not in base:
        raise ValueError('PostgreSQL base must remain digest-pinned')
    (state / 'Tools.Dockerfile').write_text('ARG DEVELOPMENT_IMAGE\nFROM ${DEVELOPMENT_IMAGE} AS cloud-development\nFROM ' + base + '\nRUN useradd --uid 1000 --create-home --user-group noms\nCOPY --from=cloud-development /usr/local/bin/pgschema /usr/local/bin/minio /usr/local/bin/mc /usr/local/bin/\n')
    if args.refresh_proxy:
        refresh_proxy(Path(os.environ.get('DOCKER_CONFIG', str(Path.home() / '.docker'))) / 'config.json')
    print(state)


if __name__ == '__main__':
    main()
