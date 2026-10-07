"""Validate the Ord deployment contract before building or deploying."""
from pathlib import Path
import re
import subprocess

for path in Path('.github/workflows').glob('*.yml'):
    text = path.read_text()
    assert 'ubuntu-latest' not in text, path
    for runner in re.findall(r'runs-on:\s*(.+)', text):
        assert runner.strip() == 'self-hosted', (path, runner)
    # Each third-party action is pinned to an immutable revision.
    for action in re.findall(r'uses:\s*([^\s]+)', text):
        if not action.startswith('./'):
            assert re.search(r'@[0-9a-f]{40}$', action), (path, action)
for path in Path('.github/actions').rglob('action.yml'):
    for action in re.findall(r'uses:\s*([^\s]+)', path.read_text()):
        assert action.startswith('./') or re.search(r'@[0-9a-f]{40}$', action), (path, action)
for path in Path('.github/actions').rglob('*.sh'):
    subprocess.run(['bash', '-n', str(path)], check=True)
for path in [Path('entrypoint.sh'), Path('docker/healthcheck.sh'), Path('scripts/restart_ord.sh')]:
    subprocess.run(['sh', '-n', str(path)], check=True)
compose = Path('docker/compose.template.yml').read_text()
assert 'source: /home/ord/ord_db' in compose
assert 'target: /app/ord_db' in compose
assert 'create_host_path: false' in compose
assert 'stop_signal: SIGTERM' in compose and 'stop_grace_period: 15m' in compose
assert 'ORD_INDEX_CACHE_SIZE' in Path('.env.template').read_text()
for path in Path('.github/actions').rglob('*.sh'):
    text = path.read_text()
    assert not re.search(r'docker\s+(?:rm\s+-f|kill[^\n]*(?:SIGKILL|--signal[= ]9))', text), path
print('Ord deployment contract passed')
