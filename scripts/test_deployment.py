"""Exercise the real stop routine with a Docker process simulator."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path('.github/actions/deploy-over-ssh/deploy.sh').read_text()
STOP = 'stop_cleanly() {' + SCRIPT.split('stop_cleanly() {', 1)[1].split('\n}', 1)[0] + '\n}'
DOCKER = '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
p = Path(os.environ['FIXTURE'])
a = sys.argv[1:]
with (p/'calls').open('a') as f: f.write(json.dumps(a)+'\\n')
mode = os.environ['MODE']
if a[0] == 'inspect':
    if '.State.Running' in a[2]:
        print('false' if mode == 'stopped' or (mode != 'stuck' and (p/'signalled').exists()) else 'true')
    else:
        print('137 true' if mode == 'oom' else '0 false')
elif a[0] == 'kill':
    assert a[1:] == ['--signal=SIGTERM', 'bells-mainnet-ord']
    (p/'signalled').touch()
elif a[0] == 'update':
    assert a[1:] == ['--restart=no', 'bells-mainnet-ord']
else:
    raise RuntimeError('Unexpected destructive operation: '+repr(a))
'''

class StopTests(unittest.TestCase):
    def run_stop(self, mode):
        with tempfile.TemporaryDirectory() as temp:
            p = Path(temp)
            (p/'docker').write_text(DOCKER)
            (p/'docker').chmod(0o755)
            state = p/'index.redb'
            state.write_bytes(b'nonempty index fixture')
            env = dict(os.environ, PATH=str(p)+os.pathsep+os.environ['PATH'],
                       FIXTURE=str(p), MODE=mode, TRANSACTION=str(p),
                       CONTAINER='bells-mainnet-ord', STOP_TIMEOUT_SECONDS='1')
            result = subprocess.run(['bash', '-c', 'set -Eeuo pipefail\n'+STOP+'\nstop_cleanly'],
                                    env=env, capture_output=True, text=True, timeout=10)
            calls = [json.loads(line) for line in (p/'calls').read_text().splitlines()]
            self.assertEqual(state.read_bytes(), b'nonempty index fixture')
            return result, calls, (p/'shutdown-blocked').exists()

    def test_clean_exit_before_recreate(self):
        result, calls, blocked = self.run_stop('clean')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(['kill', '--signal=SIGTERM', 'bells-mainnet-ord'], calls)
        self.assertFalse(blocked)

    def test_timeout_never_kills_or_removes_state(self):
        result, calls, blocked = self.run_stop('stuck')
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(blocked)
        self.assertEqual([c for c in calls if c[0] == 'kill'],
                         [['kill', '--signal=SIGTERM', 'bells-mainnet-ord']])
        self.assertFalse(any(c[0] in ['rm', 'compose', 'start'] for c in calls))

    def test_oom_exit_requires_review(self):
        result, _, blocked = self.run_stop('oom')
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(blocked)

    def test_previously_stopped_container_is_not_started(self):
        result, calls, blocked = self.run_stop('stopped')
        self.assertEqual(result.returncode, 0)
        self.assertFalse(blocked)
        self.assertEqual(len(calls), 1)

if __name__ == '__main__':
    unittest.main()
