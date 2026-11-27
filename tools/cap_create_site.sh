#!/usr/bin/env bash
# Create a Cap site key non-interactively against a running capd instance.
#
#   ADMIN_KEY=<dashboard password> ./tools/cap_create_site.sh electricacl
#   # -> {"siteKey":"2186d3ff68","secretKey":"sk-..."}
#
# Put the two values into CAP_SITE_KEY and CAP_SECRET (see .env.example).
set -eu
CAP_URL=${CAP_URL:-http://localhost:3000}
ADMIN_KEY=${ADMIN_KEY:?set ADMIN_KEY}
NAME=${1:-electricacl}

LOGIN=$(curl -sS -X POST "$CAP_URL/auth/login" -H 'Content-Type: application/json' \
  -d "{\"admin_key\":\"$ADMIN_KEY\"}")
AUTH=$(printf '%s' "$LOGIN" | python -c "
import base64, json, sys
d = json.load(sys.stdin)
if not d.get('success'):
    raise SystemExit('login failed: %s' % d)
print(base64.b64encode(json.dumps({'token': d['session_token'], 'hash': d['hashed_token']}).encode()).decode())
")
curl -sS -X POST "$CAP_URL/server/keys" \
  -H "Authorization: Bearer $AUTH" -H 'Content-Type: application/json' \
  -d "{\"name\":\"$NAME\"}"
