#!/bin/sh
set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM

calls="$temporary/calls"

printf '%s\n' '#!/bin/sh' "printf '%s\\n' '1000 herald yes active' '1001 guest no online'" \
    >"$temporary/loginctl"
# The single quotes are the generated scripts' quotes; expansion must happen when those
# scripts run, not while these fixtures are being written.
# shellcheck disable=SC2016
printf '%s\n' '#!/bin/sh' 'printf '\''runuser %s\n'\'' "$*" >>"$HIGHWATER_TEST_CALLS"' \
    'shift 3' '"$@"' >"$temporary/runuser"
# shellcheck disable=SC2016
printf '%s\n' '#!/bin/sh' 'printf '\''systemctl %s\n'\'' "$*" >>"$HIGHWATER_TEST_CALLS"' \
    >"$temporary/systemctl"
chmod +x "$temporary/loginctl" "$temporary/runuser" "$temporary/systemctl"

HIGHWATER_LOGINCTL="$temporary/loginctl" \
HIGHWATER_RUNUSER="$temporary/runuser" \
HIGHWATER_SYSTEMCTL="$temporary/systemctl" \
HIGHWATER_TEST_CALLS="$calls" \
    "$project_root/data/restart-user-daemon"

sed -i "s|$temporary/systemctl|/mock/systemctl|g" "$calls"

cat >"$temporary/expected" <<'EOF'
runuser -u herald -- env XDG_RUNTIME_DIR=/run/user/1000 /mock/systemctl --user daemon-reload
systemctl --user daemon-reload
runuser -u herald -- env XDG_RUNTIME_DIR=/run/user/1000 /mock/systemctl --user try-restart highwaterd.service
systemctl --user try-restart highwaterd.service
runuser -u guest -- env XDG_RUNTIME_DIR=/run/user/1001 /mock/systemctl --user daemon-reload
systemctl --user daemon-reload
runuser -u guest -- env XDG_RUNTIME_DIR=/run/user/1001 /mock/systemctl --user try-restart highwaterd.service
systemctl --user try-restart highwaterd.service
EOF

diff -u "$temporary/expected" "$calls"
