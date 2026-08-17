#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KEEPER="$SCRIPT_DIR/maintain-contract-ttl.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cat > "$TMP_DIR/deployment.json" <<'JSON'
{
  "network": "testnet",
  "contracts": {
    "amm": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "market_registry": "CBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    "mock_usdc": ""
  }
}
JSON

output="$($KEEPER \
  --network testnet \
  --manifest "$TMP_DIR/deployment.json")"

[ "$(printf '%s\n' "$output" | grep -c 'stellar contract extend')" -eq 2 ]
printf '%s\n' "$output" | grep -q -- '--id'
printf '%s\n' "$output" | grep -q 'Dry run complete'

cat > "$TMP_DIR/keys.tsv" <<'EOF'
# contract_name durability key_xdr
amm persistent AAAA
EOF

restore_output="$($KEEPER \
  --network testnet \
  --manifest "$TMP_DIR/deployment.json" \
  --keys-file "$TMP_DIR/keys.tsv" \
  --restore)"

[ "$(printf '%s\n' "$restore_output" | grep -c 'stellar contract restore')" -eq 3 ]
printf '%s\n' "$restore_output" | grep -q -- '--key-xdr AAAA'

cat > "$TMP_DIR/temporary-keys.tsv" <<'EOF'
amm temporary AAAA
EOF

if "$KEEPER" \
  --network testnet \
  --manifest "$TMP_DIR/deployment.json" \
  --keys-file "$TMP_DIR/temporary-keys.tsv" \
  --restore >"$TMP_DIR/temporary-restore.out" 2>&1; then
  echo "expected temporary restore to fail" >&2
  exit 1
fi

if grep -q 'stellar contract restore' "$TMP_DIR/temporary-restore.out"; then
  echo "restore commands emitted before preflight completed" >&2
  exit 1
fi

if "$KEEPER" \
  --network mainnet \
  --manifest "$TMP_DIR/deployment.json" >/dev/null 2>&1; then
  echo "expected network mismatch to fail" >&2
  exit 1
fi

echo "maintain-contract-ttl dry-run tests passed"
