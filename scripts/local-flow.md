# Local Manual Flow

This file is an operator checklist rather than an executable script because contract IDs are produced at deployment time.

1. Start local network and fund `alice`.
2. Deploy `mock_usdc`; mint test USDC to curator, trader, LP, proposer, disputer.
3. Deploy all Dike modules.
4. Wire module roles and supported collateral.
5. Governance/timelock approves:
   - market creator
   - council members
   - mock USDC collateral
   - treasury address
6. Curator calls `create_market`.
7. LP seeds AMM liquidity.
8. Trader buys YES or NO.
9. After expiry, anyone calls `request_resolution`.
10. Proposer calls `propose_outcome`.
11. If undisputed, call `finalize_undisputed`.
12. If disputed, call `dispute_outcome`, `escalate_to_council`, `open_case`, `commit_vote`, `reveal_vote`, `finalize_case`, then `report_council_outcome`.
13. Holders redeem winning or invalid-refund positions through the vault/token flow.
