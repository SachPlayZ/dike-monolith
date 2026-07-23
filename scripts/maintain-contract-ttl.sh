#!/usr/bin/env bash
set -euo pipefail

NETWORK="${NETWORK:-}"
MANIFEST="${MANIFEST:-}"
SOURCE="${SOURCE:-}"
KEYS_FILE="${KEYS_FILE:-}"
LEDGERS_TO_EXTEND="${LEDGERS_TO_EXTEND:-518400}"
FEE="${FEE:-100}"
ACTION="${ACTION:-extend}"
EXECUTE="${EXECUTE:-false}"

usage() {
  cat <<'EOF'
Usage: scripts/maintain-contract-ttl.sh --network <name> [options]

Dry-runs TTL maintenance commands by default. Pass --execute to submit them.

Options:
  --network <name>       Stellar network name (must match the manifest)
  --manifest <path>      Deployment manifest (default: deployments/<network>.json)
  --source <identity>    Stellar source identity/account; required with --execute
  --keys-file <path>     Optional persistent/temporary key inventory; see docs/DEPLOYMENT.md
  --ledgers <count>      Target ledger extension (default: 518400, about 30 days)
  --fee <stroops>        Base transaction fee (default: 100)
  --restore              Restore archived entries instead of extending active entries
  --execute              Submit transactions (default is dry-run)
  -h, --help             Show this help
EOF
}

die() {
  echo "ERROR: $*" >&2
  exit 1
}

known_contract_name() {
  case "$1" in
    dike_timelock|dike_governance|market_registry|conditional_tokens|collateral_vault|amm|fee_manager|cod_oracle|council_of_dike|market_factory|mock_usdc) return 0 ;;
    *) return 1 ;;
  esac
}

print_command() {
  local arg
  printf '  '
  for arg in "$@"; do
    printf '%q ' "$arg"
  done
  printf '\n'
}

run_command() {
  if [ "$EXECUTE" = "true" ]; then
    "$@"
  else
    print_command "$@"
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --network) [ "$#" -ge 2 ] || die "--network requires a value"; NETWORK="$2"; shift 2 ;;
    --manifest) [ "$#" -ge 2 ] || die "--manifest requires a value"; MANIFEST="$2"; shift 2 ;;
    --source) [ "$#" -ge 2 ] || die "--source requires a value"; SOURCE="$2"; shift 2 ;;
    --keys-file) [ "$#" -ge 2 ] || die "--keys-file requires a value"; KEYS_FILE="$2"; shift 2 ;;
    --ledgers) [ "$#" -ge 2 ] || die "--ledgers requires a value"; LEDGERS_TO_EXTEND="$2"; shift 2 ;;
    --fee) [ "$#" -ge 2 ] || die "--fee requires a value"; FEE="$2"; shift 2 ;;
    --restore) ACTION="restore"; shift ;;
    --execute) EXECUTE="true"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

[ -n "$NETWORK" ] || die "--network is required"
[[ "$NETWORK" =~ ^[A-Za-z0-9._-]+$ ]] || die "invalid network name: $NETWORK"
[[ "$LEDGERS_TO_EXTEND" =~ ^[1-9][0-9]*$ ]] || die "--ledgers must be a positive integer"
[ "$LEDGERS_TO_EXTEND" -le 535679 ] || die "--ledgers exceeds the current Stellar CLI maximum (535679)"
[[ "$FEE" =~ ^[1-9][0-9]*$ ]] || die "--fee must be a positive integer"
[ "$ACTION" = "extend" ] || [ "$ACTION" = "restore" ] || die "ACTION must be extend or restore"
[ "$EXECUTE" = "true" ] || [ "$EXECUTE" = "false" ] || die "EXECUTE must be true or false"

MANIFEST="${MANIFEST:-deployments/$NETWORK.json}"
[ -f "$MANIFEST" ] || die "manifest not found: $MANIFEST"
command -v jq >/dev/null 2>&1 || die "jq is required"
jq -e '.network | type == "string"' "$MANIFEST" >/dev/null || die "invalid manifest: $MANIFEST"

manifest_network="$(jq -r '.network' "$MANIFEST")"
[ "$manifest_network" = "$NETWORK" ] || die "manifest network '$manifest_network' does not match '$NETWORK'"

if [ "$EXECUTE" = "true" ]; then
  [ -n "$SOURCE" ] || die "--source is required with --execute"
  command -v stellar >/dev/null 2>&1 || die "stellar CLI is required with --execute"
else
  SOURCE="${SOURCE:-<source-account>}"
fi

if [ -n "$KEYS_FILE" ]; then
  [ -f "$KEYS_FILE" ] || die "keys file not found: $KEYS_FILE"
fi

contract_count="$(jq '[.contracts | to_entries[] | select(.value != "")] | length' "$MANIFEST")"
[ "$contract_count" -gt 0 ] || die "manifest has no deployed contracts"

echo "==> Preflight: validating manifest and optional keys"
while IFS=$'\t' read -r contract_name contract_id; do
  [[ "$contract_id" =~ ^C[A-Z2-7]{55}$ ]] || die "invalid contract ID for $contract_name"
  known_contract_name "$contract_name" || die "unknown Dike contract in manifest: '$contract_name'"
done < <(jq -r '.contracts | to_entries[] | select(.value != "") | [.key, .value] | @tsv' "$MANIFEST")

if [ -n "$KEYS_FILE" ]; then
  while read -r contract_name durability key_xdr extra; do
    [ -n "${contract_name:-}" ] || continue
    case "$contract_name" in \#*) continue ;; esac
    [ -z "${extra:-}" ] || die "invalid keys-file row for $contract_name: expected 3 fields"
    [ "$durability" = "persistent" ] || [ "$durability" = "temporary" ] || die "invalid durability for $contract_name"
    if [ "$ACTION" = "restore" ] && [ "$durability" = "temporary" ]; then
      die "temporary storage cannot be restored ($contract_name)"
    fi
    [[ "$key_xdr" =~ ^[A-Za-z0-9+/]+={0,2}$ ]] || die "invalid base64 key XDR for $contract_name"

    contract_id="$(jq -r --arg name "$contract_name" '.contracts[$name] // empty' "$MANIFEST")"
    [ -n "$contract_id" ] || die "keys file references undeployed contract '$contract_name'"
    [[ "$contract_id" =~ ^C[A-Z2-7]{55}$ ]] || die "invalid contract ID for $contract_name"
  done < "$KEYS_FILE"
fi

mode_label="DRY RUN"
[ "$EXECUTE" = "true" ] && mode_label="EXECUTE"
echo "==> $mode_label: $ACTION TTL for $contract_count Dike contracts on $NETWORK"
echo "==> Manifest: $MANIFEST"
echo "==> Target: $LEDGERS_TO_EXTEND ledgers"

while IFS=$'\t' read -r contract_name contract_id; do
  echo "==> $contract_name ($contract_id): instance storage and executable WASM"
  run_command stellar contract "$ACTION" \
    --id "$contract_id" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    --ledgers-to-extend "$LEDGERS_TO_EXTEND" \
    --fee "$FEE"
done < <(jq -r '.contracts | to_entries[] | select(.value != "") | [.key, .value] | @tsv' "$MANIFEST")

if [ -n "$KEYS_FILE" ]; then
  echo "==> Processing exact storage keys from $KEYS_FILE"
  while read -r contract_name durability key_xdr extra; do
    [ -n "${contract_name:-}" ] || continue
    case "$contract_name" in \#*) continue ;; esac
    [ -z "${extra:-}" ] || die "invalid keys-file row for $contract_name: expected 3 fields"
    [ "$durability" = "persistent" ] || [ "$durability" = "temporary" ] || die "invalid durability for $contract_name"
    if [ "$ACTION" = "restore" ] && [ "$durability" = "temporary" ]; then
      die "temporary storage cannot be restored ($contract_name)"
    fi
    [[ "$key_xdr" =~ ^[A-Za-z0-9+/]+={0,2}$ ]] || die "invalid base64 key XDR for $contract_name"

    contract_id="$(jq -r --arg name "$contract_name" '.contracts[$name] // empty' "$MANIFEST")"
    [ -n "$contract_id" ] || die "keys file references undeployed contract '$contract_name'"
    [[ "$contract_id" =~ ^C[A-Z2-7]{55}$ ]] || die "invalid contract ID for $contract_name"

    echo "==> $contract_name ($contract_id): $durability storage key"
    run_command stellar contract "$ACTION" \
      --id "$contract_id" \
      --key-xdr "$key_xdr" \
      --durability "$durability" \
      --source "$SOURCE" \
      --network "$NETWORK" \
      --ledgers-to-extend "$LEDGERS_TO_EXTEND" \
      --fee "$FEE"
  done < "$KEYS_FILE"
fi

if [ "$EXECUTE" = "false" ]; then
  echo "==> Dry run complete. Review every command, then rerun with --execute."
else
  echo "==> TTL maintenance complete. Save this output in the operator log."
fi
