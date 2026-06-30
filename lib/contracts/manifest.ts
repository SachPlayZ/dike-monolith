import testnetData from "./testnet.json";

export type ContractKey =
  | "mock_usdc"
  | "dike_timelock"
  | "dike_governance"
  | "market_registry"
  | "conditional_tokens"
  | "collateral_vault"
  | "amm"
  | "fee_manager"
  | "cod_oracle"
  | "council_of_dike"
  | "market_factory";

const manifests = {
  testnet: testnetData,
} as const;

const activeNetwork =
  (process.env.NEXT_PUBLIC_DIKE_MANIFEST_NETWORK as keyof typeof manifests) ??
  "testnet";

const manifest = manifests[activeNetwork] ?? manifests.testnet;

export function getContractId(key: ContractKey): string {
  const id = manifest.contracts[key];
  if (!id) throw new Error(`Contract "${key}" not found in manifest (${activeNetwork})`);
  return id;
}

export const COLLATERAL_CONTRACT = manifest.collateral_contract;
export const ASSET_CODE = manifest.asset_code;
export const USDC_ISSUER = manifest.usdc_issuer;

// Contract IDs (lazy – call getContractId at runtime to avoid startup throws)
export const CONTRACT_IDS = {
  marketFactory: () => getContractId("market_factory"),
  marketRegistry: () => getContractId("market_registry"),
  conditionalTokens: () => getContractId("conditional_tokens"),
  collateralVault: () => getContractId("collateral_vault"),
  amm: () => getContractId("amm"),
  feeManager: () => getContractId("fee_manager"),
  codOracle: () => getContractId("cod_oracle"),
  councilOfDike: () => getContractId("council_of_dike"),
  dikeGovernance: () => getContractId("dike_governance"),
  dikeTimelock: () => getContractId("dike_timelock"),
} as const;
