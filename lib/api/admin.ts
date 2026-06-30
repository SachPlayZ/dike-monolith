import { apiGet } from "./client";
import type { AdminState, TimelockAction } from "@/lib/types";

interface GovernanceConfigResponse {
  treasury: string | null;
  fee_config_json?: {
    trading_fee_bps?: number;
    lp_fee_share_bps?: number;
    treasury_fee_share_bps?: number;
    cod_fee_share_bps?: number;
    proposal_reward?: string;
    dispute_reward?: string;
    council_reward?: string;
    creation_fee?: string;
  } | null;
}

interface GovernanceListResponse {
  kind: string;
  address: string;
  approved: boolean;
}

interface GovernanceModuleResponse {
  role: string;
  module_address: string;
}

interface GovernanceResponse {
  config: GovernanceConfigResponse | null;
  lists: GovernanceListResponse[];
  modules: GovernanceModuleResponse[];
}

export async function fetchGovernanceState(): Promise<AdminState> {
  const response = await apiGet<GovernanceResponse>("/admin/governance");
  const feeConfig = response.config?.fee_config_json;
  const approvedLists = response.lists.filter((entry) => entry.approved);

  return {
    treasury: response.config?.treasury ?? "—",
    supportedCollaterals: approvedLists
      .filter((entry) => entry.kind === "collateral")
      .map((entry) => entry.address),
    feeConfig: {
      tradingFeeBps: feeConfig?.trading_fee_bps ?? 0,
      lpFeeShareBps: feeConfig?.lp_fee_share_bps ?? 0,
      treasuryFeeShareBps: feeConfig?.treasury_fee_share_bps ?? 0,
      codFeeShareBps: feeConfig?.cod_fee_share_bps ?? 0,
      proposalReward: feeConfig?.proposal_reward ?? "0",
      disputeReward: feeConfig?.dispute_reward ?? "0",
      councilReward: feeConfig?.council_reward ?? "0",
      creationFee: feeConfig?.creation_fee ?? "0",
    },
    moduleAddresses: Object.fromEntries(
      response.modules.map((entry) => [entry.role, entry.module_address]),
    ),
    approvedCreators: approvedLists
      .filter((entry) => entry.kind === "creator")
      .map((entry) => entry.address),
    councilMembers: approvedLists
      .filter((entry) => entry.kind === "member")
      .map((entry) => entry.address),
  };
}

export async function fetchTimelockActions(): Promise<TimelockAction[]> {
  return apiGet<TimelockAction[]>("/admin/timelock");
}
