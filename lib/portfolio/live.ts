import {
  ammGetLpBalance,
  ctBalance,
  vaultGetChildDebt,
  vaultGetParentDebt,
  vaultGetRootStake,
  vaultGetUserDeposit,
} from "@/lib/contracts/clients";
import type { UserPosition } from "@/lib/types";

export async function hydratePortfolioPositions(
  address: string,
  positions: UserPosition[],
): Promise<UserPosition[]> {
  return Promise.all(
    positions.map(async (position) => {
      const [
        yesBalance,
        noBalance,
        lpShares,
        deposit,
        rootStake,
        childDebt,
        parentDebt,
      ] = await Promise.all([
        ctBalance(address, address, position.marketId, "Yes").catch(() => position.yesBalance),
        ctBalance(address, address, position.marketId, "No").catch(() => position.noBalance),
        position.poolId
          ? ammGetLpBalance(address, address, position.poolId).catch(() => position.lpShares)
          : Promise.resolve(position.lpShares),
        vaultGetUserDeposit(address, address, position.marketId).catch(() => position.deposit),
        vaultGetRootStake(address, address, position.marketId, "Yes").catch(() => position.rootStake),
        vaultGetChildDebt(address, address, position.marketId).catch(() => position.childDebt),
        vaultGetParentDebt(address, address, position.marketId, "Yes").catch(() => position.parentDebt),
      ]);

      return {
        ...position,
        yesBalance,
        noBalance,
        lpShares,
        deposit,
        rootStake,
        childDebt,
        parentDebt,
      };
    }),
  );
}
