import type { DikeManifestContracts } from "../config/manifest.js";
import { SponsorshipError, type ParsedSponsorshipTransaction } from "./types.js";

export const SPONSORED_METHODS: Readonly<Record<keyof DikeManifestContracts, readonly string[]>> = {
  mock_usdc: [],
  dike_timelock: ["execute"],
  dike_governance: [],
  market_registry: ["close_trading"],
  conditional_tokens: [],
  collateral_vault: ["redeem_resolved", "redeem_cancelled", "sweep_protocol_fees"],
  amm: [
    "buy_yes",
    "buy_no",
    "buy_child_yes",
    "buy_child_no",
    "sell_yes",
    "sell_no",
    "add_liquidity",
    "remove_liquidity",
    "claim_lp_fees",
  ],
  fee_manager: [],
  cod_oracle: [
    "request_resolution",
    "propose_outcome",
    "dispute_outcome",
    "finalize_undisputed",
    "escalate_to_council",
  ],
  council_of_dike: ["commit_vote", "reveal_vote", "finalize_and_report_case", "claim_reward"],
  market_factory: ["create_market"],
};

export function assertAllowedContractCall(
  parsed: Pick<ParsedSponsorshipTransaction, "contractId" | "method">,
  contracts: DikeManifestContracts,
) {
  const module = (Object.keys(SPONSORED_METHODS) as Array<keyof DikeManifestContracts>)
    .find((name) => contracts[name] === parsed.contractId);

  if (!module) {
    throw new SponsorshipError("DISALLOWED_CONTRACT", "The contract is not sponsored by Dike.");
  }
  if (!SPONSORED_METHODS[module].includes(parsed.method)) {
    throw new SponsorshipError("DISALLOWED_METHOD", "The contract method is not sponsored by Dike.");
  }
}
