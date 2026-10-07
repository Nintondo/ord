#!/usr/bin/env bash
# Run on the Ord host. A timeout aborts the operation instead of sending SIGKILL.
set -Eeuo pipefail
CONTAINER=bells-mainnet-ord
BASE_PATH=/opt/nintondo/bells/mainnet
SERVICE_PATH="$BASE_PATH/services/ord"
export COMPOSE_PROJECT_NAME=bells-mainnet
exec 9>"$SERVICE_PATH/.deploy.lock"
flock -w 5 9
[ ! -d "$SERVICE_PATH/.deploy-transaction" ] || { echo 'Resolve pending deployment before restarting Ord.'; exit 1; }
python3 - <<'PY'
import json, subprocess
from pathlib import Path
p = Path('/home/ord/ord_db')
assert str(p.resolve()) == str(p) and p.stat().st_dev == Path('/home').stat().st_dev != Path('/opt').stat().st_dev
c = json.loads(subprocess.check_output(['docker', 'inspect', 'bells-mainnet-ord']))[0]
assert c['Config'].get('Healthcheck') and c['Config'].get('StopSignal') == 'SIGTERM', 'Shutdown-safe image required'
assert c['State']['Running'] or (c['State']['ExitCode'] == 0 and not c['State']['OOMKilled']), 'Unclean previous exit requires review'
assert any(m['Type'] == 'bind' and m['Source'] == str(p) and m['Destination'] == '/app/ord_db' and m['RW'] for m in c['Mounts'])
PY
if [ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER")" = true ]; then
  docker update --restart=no "$CONTAINER" >/dev/null
  docker kill --signal=SIGTERM "$CONTAINER" >/dev/null
  deadline=$((SECONDS + 900))
  while [ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER")" = true ]; do
    [ "$SECONDS" -lt "$deadline" ] || { echo 'Ord is still shutting down; no forced kill sent. Inspect before retry.'; exit 1; }
    sleep 2
  done
  [ "$(docker inspect -f '{{.State.ExitCode}} {{.State.OOMKilled}}' "$CONTAINER")" = '0 false' ] || { echo 'Unclean exit; restart aborted for review.'; exit 1; }
fi
docker compose -f "$BASE_PATH/docker-compose.yml" up -d --no-deps "$CONTAINER"
deadline=$((SECONDS + 300))
while [ "$SECONDS" -lt "$deadline" ]; do
  state=$(docker inspect -f '{{.State.Status}} {{.State.Health.Status}}' "$CONTAINER")
  [ "$state" != 'running healthy' ] || { echo 'Ord restarted cleanly; the index was retained on /home.'; exit 0; }
  case "$state" in exited*|dead*|'running unhealthy') echo 'Ord failed readiness.'; exit 1 ;; esac
  sleep 3
done
echo 'Ord readiness timed out; state retained.'
exit 1
