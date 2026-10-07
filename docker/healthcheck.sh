#!/bin/sh
set -eu
exec curl --fail --silent --show-error --output /dev/null --connect-timeout 2 --max-time 4 http://127.0.0.1:3333/readyz
