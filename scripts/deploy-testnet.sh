#!/usr/bin/env bash
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:-alice}"
ADMIN="${ADMIN:-$SOURCE}"
TREASURY="${TREASURY:-$SOURCE}"
PROPOSER="${PROPOSER:-$SOURCE}"
EXECUTOR="${EXECUTOR:-$SOURCE}"
GOV_AUTH="${GOV_AUTH:-$SOURCE}"
COLLATERAL_CONTRACT="${COLLATERAL_CONTRACT:-}"
ASSET_CODE="${ASSET_CODE:-USDC}"
USDC_ISSUER="${USDC_ISSUER:-}"
ALLOW_MOCK_USDC="${ALLOW_MOCK_USDC:-false}"
DEPLOY_SAC="${DEPLOY_SAC:-false}"

MIN_DELAY="${MIN_DELAY:-0}"
GRACE_PERIOD="${GRACE_PERIOD:-604800}"
MINIMUM_BOND="${MINIMUM_BOND:-10000000}"
BOND_BPS="${BOND_BPS:-100}"
MIN_LIQUIDITY="${MIN_LIQUIDITY:-10000000}"
MIN_EXPIRY_DURATION="${MIN_EXPIRY_DURATION:-60}"

DEPLOY_DIR="${DEPLOY_DIR:-deployments}"
MANIFEST="${MANIFEST:-$DEPLOY_DIR/$NETWORK.json}"
ALIAS_PREFIX="${ALIAS_PREFIX:-dike-$NETWORK-opt}"
TX_SLEEP="${TX_SLEEP:-8}"
DEPLOY_RETRIES="${DEPLOY_RETRIES:-3}"
INVOKE_RETRIES="${INVOKE_RETRIES:-3}"
FEE="${FEE:-1000000}"
SKIP_BUILD="${SKIP_BUILD:-false}"

WASM_DIR="target/stellar"

if [[ "$COLLATERAL_CONTRACT" == G* ]]; then
  USDC_ISSUER="$COLLATERAL_CONTRACT"
  COLLATERAL_CONTRACT=""
fi

if [ -z "$USDC_ISSUER" ] && [ "$NETWORK" = "testnet" ]; then
  USDC_ISSUER="GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
fi

if [ -z "$COLLATERAL_CONTRACT" ] && [ -z "$USDC_ISSUER" ] && [ "$ALLOW_MOCK_USDC" != "true" ]; then
  echo "ERROR: set COLLATERAL_CONTRACT to a SAC contract address or USDC_ISSUER to a Stellar asset issuer. Set ALLOW_MOCK_USDC=true only for local/dev deployments." >&2
  exit 1
fi

mkdir -p "$DEPLOY_DIR"

echo "==> Building optimized Soroban WASM"
if [ "$SKIP_BUILD" = "true" ]; then
  echo "==> Skipping build because SKIP_BUILD=true"
else
  stellar contract build --optimize --out-dir "$WASM_DIR"
fi

echo "==> Funding $SOURCE on $NETWORK if needed"
stellar keys fund "$SOURCE" --network "$NETWORK" >/dev/null || true
for signer in "$ADMIN" "$PROPOSER" "$EXECUTOR" "$GOV_AUTH"; do
  stellar keys fund "$signer" --network "$NETWORK" >/dev/null || true
done

deploy() {
  local name="$1"
  local wasm="$2"
  local alias="$ALIAS_PREFIX-$name"
  local existing
  local salt
  shift 2

  existing="$(stellar contract alias show "$alias" 2>/dev/null | head -n 1 || true)"
  if [ -n "$existing" ]; then
    echo "==> Reusing $name at $existing" >&2
    printf "%s\n" "$existing"
    return
  fi

  salt="$(printf "%s" "$alias" | shasum -a 256 | awk '{print $1}')"

  local attempt=1
  while [ "$attempt" -le "$DEPLOY_RETRIES" ]; do
    echo "==> Deploying $name with salt $salt (attempt $attempt/$DEPLOY_RETRIES)" >&2
    if stellar contract deploy \
      --source "$SOURCE" \
      --network "$NETWORK" \
      --fee "$FEE" \
      --alias "$alias" \
      --salt "$salt" \
      --wasm "$wasm" \
      -- "$@" >&2; then
      sleep "$TX_SLEEP"
      stellar contract alias show "$alias" 2>/dev/null | head -n 1
      return
    fi
    sleep "$TX_SLEEP"
    existing="$(stellar contract alias show "$alias" 2>/dev/null | head -n 1 || true)"
    if [ -n "$existing" ]; then
      echo "==> Recovered $name at $existing" >&2
      printf "%s\n" "$existing"
      return
    fi
    attempt=$((attempt + 1))
  done

  echo "ERROR: failed to deploy $name after $DEPLOY_RETRIES attempts" >&2
  return 1
}

invoke_as() {
  local invoke_source="$1"
  local id="$2"
  local fn="$3"
  shift 3

  local attempt=1
  while [ "$attempt" -le "$INVOKE_RETRIES" ]; do
    echo "==> Invoking $fn on $id as $invoke_source (attempt $attempt/$INVOKE_RETRIES)"
    if stellar contract invoke \
      --id "$id" \
      --source "$invoke_source" \
      --network "$NETWORK" \
      --fee "$FEE" \
      -- "$fn" "$@"; then
      sleep "$TX_SLEEP"
      return
    fi
    sleep "$TX_SLEEP"
    attempt=$((attempt + 1))
  done

  echo "ERROR: failed to invoke $fn on $id after $INVOKE_RETRIES attempts" >&2
  return 1
}

MOCK_USDC=""
if [ -n "$USDC_ISSUER" ]; then
  ASSET_ID="$ASSET_CODE:$USDC_ISSUER"
  COLLATERAL_CONTRACT="$(stellar contract id asset --asset "$ASSET_ID" --network "$NETWORK")"
  echo "==> Using $ASSET_ID SAC at $COLLATERAL_CONTRACT"
  if [ "$DEPLOY_SAC" = "true" ]; then
    stellar contract asset deploy \
      --asset "$ASSET_ID" \
      --source-account "$SOURCE" \
      --network "$NETWORK" \
      --fee "$FEE" \
      --alias "$ALIAS_PREFIX-$ASSET_CODE-sac" >/dev/null
  fi
elif [ -z "$COLLATERAL_CONTRACT" ]; then
  MOCK_USDC="$(deploy mock-usdc "$WASM_DIR/mock_usdc.wasm" --admin "$ADMIN")"
  COLLATERAL_CONTRACT="$MOCK_USDC"
fi

TIMELOCK="$(deploy dike-timelock "$WASM_DIR/dike_timelock.wasm" --admin "$ADMIN" --proposer "$PROPOSER" --executor "$EXECUTOR" --min-delay "$MIN_DELAY" --grace-period "$GRACE_PERIOD")"
GOVERNANCE="$(deploy dike-governance "$WASM_DIR/dike_governance.wasm" --admin "$ADMIN" --timelock "$TIMELOCK" --treasury "$TREASURY")"
REGISTRY="$(deploy market-registry "$WASM_DIR/market_registry.wasm" --admin "$ADMIN")"
TOKENS="$(deploy conditional-tokens "$WASM_DIR/conditional_tokens.wasm" --admin "$ADMIN")"
VAULT="$(deploy collateral-vault "$WASM_DIR/collateral_vault.wasm" --admin "$ADMIN" --treasury "$TREASURY")"
AMM="$(deploy amm "$WASM_DIR/amm.wasm" --admin "$ADMIN")"
FEE_MANAGER="$(deploy fee-manager "$WASM_DIR/fee_manager.wasm" --admin "$ADMIN" --governance "$GOV_AUTH" --minimum-bond "$MINIMUM_BOND" --bond-bps "$BOND_BPS")"
ORACLE="$(deploy cod-oracle "$WASM_DIR/cod_oracle.wasm" --admin "$ADMIN")"
COUNCIL="$(deploy council-of-dike "$WASM_DIR/council_of_dike.wasm" --admin "$ADMIN")"
FACTORY="$(deploy market-factory "$WASM_DIR/market_factory.wasm" --admin "$ADMIN" --governance "$GOV_AUTH" --min-liquidity "$MIN_LIQUIDITY" --min-expiry-duration "$MIN_EXPIRY_DURATION")"

echo "==> Wiring module roles"
invoke_as "$ADMIN" "$REGISTRY" set_role --role factory --module "$FACTORY"
invoke_as "$ADMIN" "$REGISTRY" set_role --role oracle --module "$ORACLE"
invoke_as "$ADMIN" "$REGISTRY" set_role --role gov --module "$GOV_AUTH"
invoke_as "$ADMIN" "$REGISTRY" set_supported_collateral --collateral "$COLLATERAL_CONTRACT" --supported true

invoke_as "$ADMIN" "$TOKENS" set_role --role vault --module "$VAULT"
invoke_as "$ADMIN" "$TOKENS" set_role --role amm --module "$AMM"

invoke_as "$ADMIN" "$VAULT" set_role --role tokens --module "$TOKENS"
invoke_as "$ADMIN" "$VAULT" set_role --role oracle --module "$ORACLE"
invoke_as "$ADMIN" "$VAULT" set_role --role amm --module "$AMM"
invoke_as "$ADMIN" "$VAULT" set_role --role gov --module "$GOV_AUTH"
invoke_as "$ADMIN" "$VAULT" set_role --role registry --module "$REGISTRY"

invoke_as "$ADMIN" "$AMM" set_role --role factory --module "$FACTORY"
invoke_as "$ADMIN" "$AMM" set_role --role gov --module "$GOV_AUTH"
invoke_as "$ADMIN" "$AMM" set_modules --vault "$VAULT" --tokens "$TOKENS" --collateral "$COLLATERAL_CONTRACT" --registry "$REGISTRY"

invoke_as "$ADMIN" "$ORACLE" set_role --role gov --module "$GOV_AUTH"
invoke_as "$ADMIN" "$ORACLE" set_role --role council --module "$COUNCIL"
invoke_as "$ADMIN" "$ORACLE" set_role --role registry --module "$REGISTRY"
invoke_as "$ADMIN" "$ORACLE" set_role --role vault --module "$VAULT"

invoke_as "$ADMIN" "$COUNCIL" set_role --role gov --module "$GOV_AUTH"
invoke_as "$ADMIN" "$COUNCIL" set_role --role oracle --module "$ORACLE"

invoke_as "$ADMIN" "$FACTORY" set_modules --registry "$REGISTRY" --tokens "$TOKENS" --vault "$VAULT" --amm "$AMM" --fee-manager "$FEE_MANAGER"
invoke_as "$GOV_AUTH" "$FACTORY" set_creator --creator "$SOURCE" --approved true
invoke_as "$GOV_AUTH" "$FACTORY" set_collateral --collateral "$COLLATERAL_CONTRACT" --supported true

cat > "$MANIFEST" <<JSON
{
  "network": "$NETWORK",
  "source": "$SOURCE",
  "admin": "$ADMIN",
  "treasury": "$TREASURY",
  "governance_authority": "$GOV_AUTH",
  "collateral_contract": "$COLLATERAL_CONTRACT",
  "asset_code": "$ASSET_CODE",
  "usdc_issuer": "$USDC_ISSUER",
  "contracts": {
    "mock_usdc": "$MOCK_USDC",
    "dike_timelock": "$TIMELOCK",
    "dike_governance": "$GOVERNANCE",
    "market_registry": "$REGISTRY",
    "conditional_tokens": "$TOKENS",
    "collateral_vault": "$VAULT",
    "amm": "$AMM",
    "fee_manager": "$FEE_MANAGER",
    "cod_oracle": "$ORACLE",
    "council_of_dike": "$COUNCIL",
    "market_factory": "$FACTORY"
  }
}
JSON

echo "==> Deployment complete"
echo "Manifest: $MANIFEST"
cat "$MANIFEST"
