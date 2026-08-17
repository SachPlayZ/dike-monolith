# PLAN.md

# Dike Protocol Smart Contract Plan

## 0. Project Goal

Dike Protocol is a binary prediction market system where users can create, trade, provide liquidity to, and resolve prediction markets using collateral-backed outcome tokens.

The protocol has five core pillars:

1. **Collateral-backed YES/NO outcome tokens**
2. **AMM-based market pricing**
3. **USDC-backed settlement**
4. **Optimistic resolution**
5. **Council of Dike dispute resolution**

The Council of Dike, also called **COD**, replaces the UMA-style optimistic oracle flow with a Dike-native resolution layer.

The smart contract system must support:

- Curated market creation
- YES/NO outcome token minting
- USDC collateral custody
- AMM trading
- LP liquidity
- Market expiry
- Optimistic answer proposals
- Disputes with matching bonds
- Council escalation
- Final outcome settlement
- Winner redemption
- Protocol fees
- Governance-controlled upgrades and parameters

---

# 1. Core Contract Modules

The smart contract system is divided into these main modules:

```txt
DikeMarketFactory
DikeMarketRegistry
DikeConditionalTokens
CollateralVault
DikeAMM
FeeManager
CODOracle
CouncilOfDike
DikeGovernance
DikeTimelock
```

Each module should have one clear responsibility.

The most important design rule:

> No single contract should secretly control all funds, outcomes, trading, and resolution.

The system should be modular so each part can be tested, audited, upgraded, and reasoned about separately.

---

# 2. High-Level Contract Architecture

```txt
User
 |
 | create market / trade / provide liquidity / propose / dispute / redeem
 v
DikeMarketFactory
 |
 | creates market configuration
 v
DikeMarketRegistry
 |
 | stores market state and metadata
 v
DikeConditionalTokens
 |
 | creates YES / NO outcome token positions
 v
CollateralVault
 |
 | holds USDC collateral and pays redemptions
 v
DikeAMM
 |
 | handles pricing, trades, and LP liquidity
 v
CODOracle
 |
 | handles optimistic proposal and disputes
 v
CouncilOfDike
 |
 | resolves disputed markets
 v
CollateralVault
 |
 | pays winners after final outcome
```

---

# 3. Build Philosophy

The protocol must prioritize:

```txt
Solvency
Clear state transitions
No unbacked outcome tokens
Simple v1 scope
Reliable dispute resolution
Visible rules before trading
Governance-controlled upgrades
Safe redemption
```

The protocol should avoid in v1:

```txt
Permissionless market creation
Native three-outcome AMMs
Orderbook trading
Token-weighted governance voting
Cross-chain settlement
Leveraged prediction positions
Recursive prediction chaining
Borrowing against outcome tokens
Complex margin systems
```

---

# 4. Core Data Concepts

## 4.1 Market

A market is one prediction question with two tradable outcomes:

```txt
YES
NO
```

Example:

```txt
Will Strait of Hormuz traffic return to normal by end of June 2026?
```

Each market must define:

```txt
Market ID
Question
Rules URI
Expiry timestamp
Collateral asset
YES token ID
NO token ID
AMM pool ID
Bond amount
Dispute window
Market creator
Market status
Final outcome
Creation timestamp
Fee config
Resolution request ID
```

---

## 4.2 Outcome Tokens

Every binary market has two outcome tokens:

```txt
YES token
NO token
```

A complete set is:

```txt
1 YES + 1 NO
```

A complete set is backed by:

```txt
1 unit of collateral
```

For example:

```txt
1 YES + 1 NO = 1 USDC backing
```

After resolution:

```txt
If YES wins:
YES = 1 USDC
NO = 0 USDC

If NO wins:
NO = 1 USDC
YES = 0 USDC
```

If the market is invalid:

```txt
YES = 0.5 USDC
NO = 0.5 USDC
```

For v1, invalid markets should use refund-style logic instead of a native INVALID token.

---

## 4.3 Collateral

The v1 collateral asset should be:

```txt
USDC
```

All markets should be backed by USDC to keep redemption logic simple.

Later versions can support:

```txt
Multiple collateral assets
Cross-chain collateral
Yield-bearing collateral
Protocol-owned liquidity
```

---

## 4.4 Final Outcome

A market can resolve to:

```txt
YES
NO
INVALID
```

The final outcome is written once and cannot be changed afterward.

Important rule:

```txt
A market must never resolve twice.
```

---

# 5. Market Lifecycle

## Full Lifecycle

```txt
1. Market is created
2. Market is seeded with liquidity
3. Market goes live
4. Users trade YES / NO
5. Market reaches expiry
6. Trading closes
7. COD resolution request is created
8. Someone proposes an outcome
9. Dispute window opens
10. If no dispute, proposed answer becomes final
11. If disputed, case escalates to Council of Dike
12. Council votes
13. Council outcome becomes final
14. Winners redeem
15. Market closes permanently
```

---

# 6. Market Status State Machine

Each market should have a strict status.

```txt
CREATED
LIVE
PAUSED
TRADING_CLOSED
RESOLUTION_REQUESTED
PROPOSED
DISPUTED
COUNCIL_VOTING
RESOLVED
CANCELLED
```

## Allowed Transitions

```txt
CREATED -> LIVE

LIVE -> PAUSED
PAUSED -> LIVE

LIVE -> TRADING_CLOSED

TRADING_CLOSED -> RESOLUTION_REQUESTED

RESOLUTION_REQUESTED -> PROPOSED

PROPOSED -> RESOLVED
PROPOSED -> DISPUTED

DISPUTED -> COUNCIL_VOTING

COUNCIL_VOTING -> RESOLVED

CREATED -> CANCELLED
LIVE -> CANCELLED
PAUSED -> CANCELLED
```

## Disallowed Transitions

```txt
RESOLVED -> LIVE
RESOLVED -> PAUSED
CANCELLED -> LIVE
CANCELLED -> RESOLVED
COUNCIL_VOTING -> LIVE
COUNCIL_VOTING -> TRADE
RESOLUTION_REQUESTED -> LIVE
TRADING_CLOSED -> LIVE
```

The state machine is critical because trading, resolution, and redemption must never overlap incorrectly.

---

# 7. Contract Module Details

---

# 8. DikeMarketFactory

## Purpose

The factory is responsible for creating new markets.

It should initialize market configuration but should not hold user funds long-term.

## Responsibilities

```txt
Create new markets
Validate required parameters
Register question and rules
Assign market ID
Assign outcome token IDs
Set collateral asset
Set expiry
Set proposal bond
Set dispute window
Set market creator
Initialize AMM pool
Register market in registry
Emit creation event
```

## Required Market Creation Inputs

Every new market should include:

```txt
Question
Rules URI
Expiry timestamp
Collateral token
Initial liquidity amount
Opening price
Proposal bond amount
Dispute window duration
Market category
Creator address
Fee configuration
```

## Market Creation Rules

The factory should reject markets if:

```txt
Question is empty
Rules URI is empty
Expiry is in the past
Expiry is too soon
Bond is below minimum bond
Dispute window is too short
Collateral asset is unsupported
Initial liquidity is below minimum
Opening price is invalid
Creator is not approved in v1
```

## V1 Permission Model

For v1:

```txt
Only approved creators can create markets.
```

Approved creators can be:

```txt
Admin
Protocol multisig
Whitelisted curators
Dike governance executor
```

This prevents spam and ambiguous markets.

## V2 Permission Model

For v2:

```txt
Anyone can create a market by posting a creation bond.
```

The creation bond can be slashed if:

```txt
The market is spam
The market is malicious
The resolution rules are impossible
The source is fake
The market is intentionally vague
The market duplicates an existing market
```

---

# 9. DikeMarketRegistry

## Purpose

The registry is the canonical source of truth for market metadata and lifecycle state.

It should not perform heavy trading logic.

It should store:

```txt
Market configuration
Market status
Market outcome token IDs
Market expiry
Market resolution configuration
Final outcome
```

## Responsibilities

```txt
Store market data
Track market status
Track final outcome
Expose market metadata
Authorize status transitions
Block invalid state changes
Expose status to other modules
```

## Stored Data Per Market

```txt
Market ID
Question
Rules URI
Creator
Collateral asset
YES token ID
NO token ID
Expiry
Status
Final outcome
AMM pool ID
Bond amount
Dispute window
Resolution request ID
Creation timestamp
Trading fee config
Resolution fee config
```

## Write Access Rules

Only specific modules should be able to update specific fields.

```txt
Factory:
- Can register new markets

AMM:
- Can read market status
- Cannot change final outcome

CODOracle:
- Can move market into resolution states
- Can finalize undisputed outcomes

CouncilOfDike:
- Cannot directly modify markets
- Must report final answer through CODOracle

Governance:
- Can pause/cancel markets under strict conditions

Registry:
- Enforces transition validity
```

## Final Outcome Rules

The registry should only accept a final outcome if:

```txt
Market is in valid resolution state
Outcome is YES / NO / INVALID
Outcome has not already been set
Caller is authorized
Market has not been cancelled
```

Once set:

```txt
Final outcome is immutable.
```

---

# 10. DikeConditionalTokens

## Purpose

This module manages collateral-backed outcome tokens.

Each market receives:

```txt
YES token
NO token
```

The tokens represent claims on the market collateral after resolution.

## Responsibilities

```txt
Create outcome token IDs
Mint YES / NO complete sets
Burn YES / NO complete sets
Allow transfer of outcome tokens
Support merging back into collateral
Support redemption after final outcome
Expose balances
Prevent unbacked minting
```

## Complete Set Creation

When collateral is split:

```txt
User deposits 1 USDC
System mints 1 YES and 1 NO
```

The complete set is fully collateralized.

```txt
1 YES + 1 NO = 1 USDC
```

## Complete Set Merge

Before resolution, a user can merge equal amounts of YES and NO back into collateral.

Flow:

```txt
User returns equal YES and NO
YES and NO are burned
Vault releases equivalent USDC
```

This lets arbitrageurs restore price balance and keeps the market efficient.

## Redemption After Resolution

After final outcome:

```txt
Winning tokens redeem for USDC
Losing tokens redeem for zero
```

For invalid markets:

```txt
YES and NO redeem for equal refund value
```

## Token Minting Rules

Outcome tokens can only be minted when:

```txt
Collateral has been deposited
Market exists
Market is live or being seeded
Market is not resolved
Market is not cancelled
Minting amount is valid
```

## Token Burning Rules

Outcome tokens can be burned when:

```txt
User merges complete sets
User redeems winning tokens
User burns worthless losing tokens
AMM removes liquidity
Market is cancelled and refunded
```

## Critical Invariants

```txt
No YES/NO tokens without collateral
Complete sets must always be collateral-backed
Winning tokens cannot redeem before resolution
Losing tokens cannot redeem collateral
Invalid markets must have deterministic refund logic
Cancelled markets must allow fair recovery
```

---

# 11. CollateralVault

## Purpose

The vault holds all collateral backing markets.

It is the most important financial safety component.

## Responsibilities

```txt
Receive USDC deposits
Hold market collateral
Track collateral per market
Release collateral on merge
Pay winners after resolution
Track redeemed amounts
Track protocol fees
Prevent over-redemption
Protect solvency
```

## Per-Market Accounting

For each market, the vault should track:

```txt
Total collateral deposited
Collateral backing complete sets
Collateral allocated to AMM
Collateral already redeemed
Protocol fees collected
LP fees collected
Refundable collateral
Maximum redeemable collateral
```

## Deposit Flow

```txt
User sends USDC
Vault receives USDC
Conditional token module mints YES / NO
Collateral balance increases
```

## Merge Flow

```txt
User returns equal YES and NO
Outcome tokens are burned
Vault releases USDC
Collateral balance decreases
```

## Redemption Flow

```txt
Market is resolved
User submits winning outcome token
Winning token is burned
Vault pays USDC
Redeemed amount is recorded
```

## Invalid Market Refund Flow

If market resolves INVALID:

```txt
User submits YES or NO tokens
Vault pays refund value
Tokens are burned
Redeemed amount is recorded
```

Suggested v1 logic:

```txt
YES token = 0.5 USDC
NO token = 0.5 USDC
```

## Cancellation Flow

If a market is cancelled before resolution:

```txt
Trading stops
Users can merge complete sets
LPs can withdraw proportional collateral
No YES/NO winner is selected
Vault releases collateral based on cancellation accounting
```

## Vault Safety Rules

The vault must reject withdrawals if:

```txt
Market does not exist
Market is not in correct state
User has insufficient outcome tokens
Requested redemption exceeds entitlement
Market collateral is insufficient
Caller is unauthorized
Token amount is zero
```

## Core Vault Invariant

```txt
Market collateral must always be greater than or equal to maximum possible redeemable claims.
```

In plain language:

> The vault should always be able to pay winners.

---

# 12. DikeAMM

## Purpose

The AMM handles pricing, buying, selling, and liquidity.

For v1, Dike should use an AMM instead of an orderbook.

## Responsibilities

```txt
Create market pools
Seed initial liquidity
Buy YES
Buy NO
Sell YES
Sell NO
Add liquidity
Remove liquidity
Track pool reserves
Track LP shares
Charge trading fees
Calculate price impact
Enforce slippage protection
Prevent trading after expiry
```

## AMM Pricing Model

Use a fixed-product style prediction market AMM.

```txt
YES reserve × NO reserve = constant
```

Price intuition:

```txt
If YES reserve decreases, YES becomes more expensive.
If NO reserve decreases, NO becomes more expensive.
```

Simple price view:

```txt
YES price = NO reserve / total reserves
NO price = YES reserve / total reserves
```

Example:

```txt
YES reserve = 5,000
NO reserve = 5,000

YES price = 0.50
NO price = 0.50
```

After many users buy YES:

```txt
YES reserve decreases
NO reserve increases
YES price increases
NO price decreases
```

## Liquidity Seeding

Initial liquidity flow:

```txt
LP deposits USDC
Vault receives USDC
Conditional token module mints YES and NO
AMM receives YES and NO reserves
LP receives LP shares
Market becomes tradeable
```

Example:

```txt
LP deposits 5,000 USDC

System mints:
5,000 YES
5,000 NO

AMM starts with:
YES reserve = 5,000
NO reserve = 5,000

Opening price:
YES = 0.50
NO = 0.50
```

## Buying YES

Flow:

```txt
User pays USDC
Vault receives collateral
System mints complete set
AMM receives opposite side exposure
User receives YES tokens
AMM reserves update
YES price increases
```

The user should see:

```txt
Input USDC
Expected YES received
Average price
Price impact
Trading fee
Minimum output
```

## Buying NO

Flow:

```txt
User pays USDC
Vault receives collateral
System mints complete set
AMM receives opposite side exposure
User receives NO tokens
AMM reserves update
NO price increases
```

## Selling YES

Flow:

```txt
User sends YES tokens
AMM updates reserves
User receives USDC
YES price decreases
```

## Selling NO

Flow:

```txt
User sends NO tokens
AMM updates reserves
User receives USDC
NO price decreases
```

## LP Shares

LP shares represent ownership of the AMM pool.

The AMM should track:

```txt
Total LP shares
LP share balances
Pool reserves
Accumulated LP fees
Protocol fees
Liquidity added
Liquidity removed
```

## Adding Liquidity

Flow:

```txt
LP deposits collateral
Collateral is split into YES / NO
YES and NO are added to the pool
LP receives pool shares
```

## Removing Liquidity

Flow:

```txt
LP burns pool shares
LP receives proportional pool assets
LP may receive YES and NO
LP may merge complete sets if possible
```

Before resolution, LP withdrawal may return:

```txt
YES tokens
NO tokens
Some collateral
```

After resolution, LPs redeem whatever winning exposure remains.

## Trading Safety

Every trade must enforce:

```txt
Minimum output amount
Maximum acceptable price impact
Deadline
Market must be live
Market must not be expired
Market must not be paused
Market must not be in resolution
Market must not be resolved
```

## AMM Invariants

```txt
Reserves cannot go negative
Pool cannot trade after expiry
Pool cannot trade after resolution request
Fees must be accounted correctly
LP shares must represent proportional ownership
Trading must not mint unbacked outcome tokens
Pool must not drain vault collateral incorrectly
```

---

# 13. FeeManager

## Purpose

The FeeManager centralizes protocol fee logic.

Fees should not be hardcoded across multiple contracts.

## Responsibilities

```txt
Store fee configuration
Calculate trading fees
Calculate protocol cut
Calculate LP cut
Calculate COD reward cut
Calculate market creation fee
Calculate proposal reward
Calculate council reward
Route fees to correct destinations
Allow governance-controlled fee changes
```

## Fee Types

```txt
Trading fee
LP fee
Protocol fee
Oracle reward fee
Market creation fee
Proposal reward
Dispute reward
Council reward
Treasury fee
```

## Suggested Trading Fee Split

```txt
Trading fee = 2%

70% -> LPs
20% -> protocol treasury
10% -> COD reward pool
```

## Market Creation Fee

For v1:

```txt
Market creation fee can be zero if creation is permissioned.
```

For v2:

```txt
Market creator pays creation fee
Market creator posts creation bond
Fee goes to treasury
Bond can be slashed for invalid/spam markets
```

## Proposal Reward

When someone proposes a valid answer and no dispute occurs:

```txt
Proposer gets bond back
Proposer receives proposal reward
```

This incentivizes users to resolve expired markets.

## Fee Governance

Governance can update:

```txt
Trading fee percentage
LP fee split
Treasury fee split
COD reward percentage
Minimum proposal bond
Bond percentage
Proposal reward amount
Council reward split
Creation fee
```

All fee changes should pass through a timelock.

---

# 14. CODOracle

## Purpose

CODOracle is the Council of Dike optimistic oracle adapter.

It handles:

```txt
Resolution requests
Outcome proposals
Disputes
No-dispute finalization
Escalation to Council of Dike
Final outcome reporting
```

It is the bridge between market lifecycle and the Council of Dike.

---

## CODOracle Request Data

Each resolution request should store:

```txt
Request ID
Market ID
Question hash
Rules URI
Expiry timestamp
Request timestamp
Bond amount
Dispute window
Proposer
Proposed outcome
Proposal evidence URI
Proposal timestamp
Disputer
Disputed outcome
Dispute evidence URI
Dispute timestamp
Oracle status
Final outcome
```

## Oracle Statuses

```txt
NONE
REQUESTED
PROPOSED
DISPUTED
ESCALATED
FINALIZED
```

---

# 15. COD Resolution Flow

## Step 1: Request Created

After market expiry, anyone can request resolution.

Request stores:

```txt
Market ID
Question hash
Rules URI
Expiry timestamp
Bond amount
Dispute window duration
Request timestamp
```

Market status changes:

```txt
TRADING_CLOSED -> RESOLUTION_REQUESTED
```

Rules:

```txt
Market must exist
Market must be expired
Market must not be resolved
Market must not be cancelled
Resolution must not already be requested
Trading must be closed
```

---

## Step 2: Proposal

Anyone can propose an answer.

Proposal includes:

```txt
Proposed outcome
Evidence URI
Proposer address
Bond amount
Proposal timestamp
```

Possible proposed outcomes:

```txt
YES
NO
INVALID
```

The proposer must post the required bond.

Example:

```txt
Proposed outcome: YES
Bond: 500 USDC
Evidence URI: archived official source
```

Market status changes:

```txt
RESOLUTION_REQUESTED -> PROPOSED
```

The dispute timer starts.

Rules:

```txt
Proposal must happen after resolution request
Proposed outcome must be valid
Bond must be posted before proposal is accepted
Evidence URI must not be empty
Market must not already have a proposal
```

---

## Step 3: Dispute Window

Anyone can dispute during the dispute window.

Dispute includes:

```txt
Counter outcome
Evidence URI
Disputer address
Matching bond
Dispute timestamp
```

The disputer posts a matching bond.

Example:

```txt
Proposer says YES with 500 USDC bond.
Disputer says NO with 500 USDC bond.
```

Market status changes:

```txt
PROPOSED -> DISPUTED -> COUNCIL_VOTING
```

Rules:

```txt
Dispute must happen before dispute window ends
Disputer must post matching bond
Counter outcome must be valid
Counter outcome should differ from proposed outcome
Evidence URI must not be empty
Market must currently be in proposed state
```

---

## Step 4A: No Dispute

If the dispute window ends and no dispute occurs:

```txt
Proposed answer becomes final.
```

Market status changes:

```txt
PROPOSED -> RESOLVED
```

Proposer receives:

```txt
Original bond
Proposal reward
```

Rules:

```txt
Dispute window must have ended
No dispute must exist
Market must not already be resolved
Only the proposed outcome can be finalized
```

---

## Step 4B: Dispute

If a dispute occurs:

```txt
The case escalates to Council of Dike.
```

CODOracle creates or references a Council case.

Market status changes:

```txt
DISPUTED -> COUNCIL_VOTING
```

The Council decides:

```txt
YES
NO
INVALID
```

Once Council finalizes, CODOracle records the final outcome and updates the market registry.

---

# 16. CouncilOfDike

## Purpose

CouncilOfDike is the final dispute court for Dike markets.

It replaces the DVM-style process with a Dike-native council.

## Responsibilities

```txt
Receive disputed cases
Open council voting
Store evidence references
Manage voting periods
Collect council votes
Finalize vote outcome
Redistribute bonds
Reward correct voters
Report final outcome to CODOracle
```

---

## Council Case Data

Each council case should store:

```txt
Case ID
Market ID
Question
Rules URI
Proposer
Proposer outcome
Proposer evidence URI
Disputer
Disputer outcome
Disputer evidence URI
Proposal bond
Dispute bond
Voting start time
Commit phase end time
Reveal phase end time
Case status
Final outcome
Winning side
Total valid votes
```

## Council Case Statuses

```txt
OPENED
COMMIT_PHASE
REVEAL_PHASE
READY_TO_FINALIZE
FINALIZED
CANCELLED
```

---

# 17. Council Voting Model

The Council should use commit-reveal voting.

This prevents voters from copying others and reduces voting manipulation.

## Phase 1: Commit

Council members submit hidden votes.

The hidden vote commits to:

```txt
Selected outcome
Secret
Council member identity
Case ID
```

No one can see the selected outcome during this phase.

## Phase 2: Reveal

Council members reveal:

```txt
Selected outcome
Secret
```

The system verifies that the reveal matches the earlier commit.

Valid votes are counted.

Invalid reveals are ignored or penalized.

## Phase 3: Finalize

After reveal phase ends:

```txt
Votes are counted
Majority outcome is selected
Final outcome is stored
Bonds are redistributed
Council voters are rewarded
Outcome is reported to CODOracle
```

---

# 18. Council Membership Model

## V1: Whitelisted Council

Use this for testnet and early product.

```txt
Council members are approved by admin/governance.
Only council members can vote.
Council members can be added or removed.
Council members receive rewards for correct votes.
```

Benefits:

```txt
Fast to build
Easy to demo
Easy to reason about
Good for hackathon/testnet
```

Downsides:

```txt
More centralized
Requires trust in selected council members
```

## V2: Staked Council

Council members stake DIKE or COD tokens.

```txt
Members stake to join
Correct voting earns rewards
Incorrect voting reduces reputation
Malicious behavior can be slashed
Inactive members can be removed
```

## V3: Token-Weighted Dike DVM

A future version can allow wider token-holder voting.

This is more decentralized but harder because it introduces:

```txt
Bribery risk
Low turnout
Vote buying
Delegation problems
Longer resolution cycles
Governance attacks
```

Recommended roadmap:

```txt
V1: Whitelisted Council
V2: Staked + reputation-weighted Council
V3: Token-weighted Dike DVM
```

---

# 19. Bond System

## Purpose

Bonds are used to make the resolution process economically secure.

They prevent:

```txt
Lazy proposals
Wrong proposals
Spam disputes
Griefing attacks
Low-cost manipulation
```

## Proposal Bond

A proposer must post a bond when submitting an answer.

The bond is returned if:

```txt
No one disputes
The Council agrees with the proposer
```

The bond is lost if:

```txt
The Council sides against the proposer
```

## Dispute Bond

A disputer must post a matching bond when challenging an answer.

The dispute bond is returned if:

```txt
The Council agrees with the disputer
```

The dispute bond is lost if:

```txt
The Council sides against the disputer
```

## Bond Sizing

Do not use one small fixed bond for all markets.

Recommended formula:

```txt
Required bond = max(minimum bond, market liquidity percentage)
```

Example:

```txt
Minimum bond = 500 USDC
Bond percentage = 1%
Market liquidity = 100,000 USDC

Required bond = 1,000 USDC
```

This prevents someone from cheaply attacking a large market.

---

# 20. Bond Redistribution

## Undisputed Case

If no dispute happens:

```txt
Proposer receives:
- Original proposal bond
- Proposal reward
```

## Disputed Case: Proposer Wins

If the Council agrees with the proposer:

```txt
Proposer receives:
- Original proposal bond
- Portion of dispute bond
- Optional proposal reward
```

Correct council voters receive:

```txt
Portion of losing bond
Council reward
```

Protocol treasury receives:

```txt
Small percentage of losing bond
```

## Disputed Case: Disputer Wins

If the Council agrees with the disputer:

```txt
Disputer receives:
- Original dispute bond
- Portion of proposal bond
- Optional dispute reward
```

Correct council voters receive:

```txt
Portion of losing bond
Council reward
```

Protocol treasury receives:

```txt
Small percentage of losing bond
```

## Suggested Losing Bond Split

```txt
60% -> Winning proposer/disputer
30% -> Correct council voters
10% -> Protocol treasury
```

---

# 21. Resolution Rules Registry

## Purpose

Prediction markets become dangerous when the question is vague.

The rules registry stores the resolution metadata reference and optionally validates that required fields exist.

The full rules should be stored off-chain on IPFS or Arweave, while the contract stores the URI and hash.

## Required Rule Fields

Each market should have a rules document containing:

```txt
Question
Outcome options
Primary source
Primary metric
YES condition
NO condition
Expiry timestamp
Timezone
Data delay rule
Fallback source
Fallback rule
Invalid condition
Dispute window
Bond amount
```

## Example Rules Document

```txt
Question:
Will Strait of Hormuz traffic return to normal by end of June 2026?

Primary source:
IMF PortWatch

Metric:
7-day moving average of transit calls

YES condition:
YES if IMF PortWatch reports transit calls equal to or above 60 on any date before June 30, 2026 23:59 UTC.

NO condition:
NO if the YES condition is not met.

Fallback rule:
If IMF PortWatch is unavailable for 14 calendar days after expiry, use the latest available data before the outage.

Invalid condition:
INVALID if the primary metric is discontinued and no fallback source exists.

Timezone:
UTC
```

## Contract-Level Requirement

The smart contracts do not need to understand every field semantically.

But they should store:

```txt
Rules URI
Rules hash
Question hash
Expiry
Bond
Dispute window
```

This prevents market rules from being silently changed after launch.

---

# 22. Emergency Controls

## Pause System

The protocol should support emergency pause.

Pauseable actions:

```txt
Market creation
Trading
Liquidity add
Liquidity remove
Resolution request
Proposal
Dispute
Council voting
Redemption
```

Redemption should only be paused in extreme cases.

A resolved market’s redemption should usually stay available unless there is a critical exploit.

## Market Cancellation

Markets can be cancelled before resolution if:

```txt
The question is broken
Rules are impossible
Source is invalid
Expiry was configured incorrectly
Market is duplicated
Exploit is detected
```

Cancellation should not select YES or NO.

Instead:

```txt
Trading stops
Users recover collateral through merge/refund
LPs withdraw proportional assets
Market status becomes CANCELLED
```

Cancellation should require:

```txt
Admin multisig
Governance approval
Timelock for non-emergency cancellation
```

---

# 23. Governance and Timelock

## Purpose

Governance controls sensitive protocol parameters.

## Governed Parameters

```txt
Trading fee
LP fee split
Protocol fee split
COD reward split
Minimum bond
Bond percentage
Proposal reward
Council reward
Market creation fee
Approved market creators
Council member list
Treasury address
Supported collateral assets
Minimum liquidity
Minimum expiry duration
Maximum dispute window
Emergency pause authority
Contract upgrade authority
```

## Timelock Requirements

Critical changes should not be instant.

Recommended:

```txt
Testnet:
12 to 24 hour timelock

Early mainnet:
48 hour timelock

Mature mainnet:
3 to 7 day timelock
```

## High-Risk Changes

These should always require timelock:

```txt
Changing vault permissions
Changing treasury address
Changing AMM logic
Changing CODOracle logic
Changing CouncilOfDike logic
Changing fee receiver
Changing supported collateral
Changing council voting rules
```

---

# 24. Upgradeability Plan

Not every module should be upgradeable.

## Upgradeable Modules

```txt
DikeMarketFactory
DikeMarketRegistry
CODOracle
CouncilOfDike
FeeManager
```

Reason:

```txt
These modules may need iteration as the protocol evolves.
```

## Restricted Upgradeability

```txt
DikeAMM
```

Reason:

```txt
Pricing logic is sensitive and directly affects user funds.
```

AMM upgrades should require:

```txt
Governance approval
Timelock
Migration plan
Public announcement
Emergency rollback plan
```

## Prefer Non-Upgradeable

```txt
CollateralVault
DikeConditionalTokens
```

Reason:

```txt
These are core solvency contracts.
Weak upgrade controls can put all user funds at risk.
```

If they are upgradeable, they must require:

```txt
Multisig
Timelock
Explicit public upgrade events
No instant upgrade path
Emergency review period
```

---

# 25. Event Plan

Events are required for the frontend, indexer, analytics, and audit trail.

## Market Events

```txt
MarketCreated
MarketStatusUpdated
MarketPaused
MarketUnpaused
MarketCancelled
MarketExpired
```

## Token Events

```txt
PositionSplit
PositionMerged
PositionRedeemed
LosingTokensBurned
```

## Vault Events

```txt
CollateralDeposited
CollateralReleased
CollateralRedeemed
VaultFeeCollected
VaultAccountingUpdated
```

## AMM Events

```txt
PoolCreated
LiquidityAdded
LiquidityRemoved
TradeExecuted
PriceUpdated
LPFeesAccrued
```

## Oracle Events

```txt
ResolutionRequested
AnswerProposed
AnswerDisputed
UndisputedFinalized
EscalatedToCouncil
FinalOutcomeReported
```

## Council Events

```txt
CouncilCaseOpened
VoteCommitted
VoteRevealed
CouncilCaseFinalized
CouncilRewardClaimed
CouncilMemberAdded
CouncilMemberRemoved
```

## Fee Events

```txt
FeeCollected
FeeDistributed
FeeConfigUpdated
TreasuryUpdated
```

## Governance Events

```txt
ParameterChangeQueued
ParameterChangeExecuted
ContractUpgradeQueued
ContractUpgradeExecuted
EmergencyPauseTriggered
EmergencyPauseLifted
```

---

# 26. Security Requirements

## Main Risks

The protocol must defend against:

```txt
Unbacked outcome token minting
Vault insolvency
Double resolution
Double redemption
Trading after expiry
Trading after resolution
Oracle griefing
Fake evidence spam
Low-cost disputes
Council bribery
Voter copying
Non-reveal griefing
MEV/sandwich attacks
Slippage manipulation
Decimal handling bugs
Fee accounting bugs
LP share inflation
Admin key compromise
Upgrade abuse
Emergency pause abuse
```

## Required Protections

```txt
Strict market state machine
No minting without collateral
No redemption before resolution
No trading after expiry
No final outcome overwrite
Slippage limits on every trade
Deadline on every trade
Reentrancy protection
Safe token transfers
Access control
Timelocked upgrades
Multisig admin
Commit-reveal council voting
Bond scaling with market liquidity
Evidence URI and hash storage
Per-market collateral accounting
```

---

# 27. Core Invariants

These are the most important rules the contracts must always preserve.

```txt
1. No outcome tokens can exist without collateral backing.

2. One complete YES/NO set is backed by one unit of collateral.

3. A market cannot be resolved more than once.

4. A market cannot trade after resolution request.

5. A market cannot trade after final resolution.

6. A market cannot be redeemed before final outcome exists.

7. Losing tokens cannot redeem collateral.

8. Winning tokens cannot redeem more than their fixed entitlement.

9. Invalid markets must refund according to deterministic rules.

10. Vault collateral must cover maximum possible claims.

11. Proposal bond must be locked before proposal is accepted.

12. Dispute bond must be locked before dispute is accepted.

13. Council cannot finalize before voting periods end.

14. Only CODOracle can report final outcome to the market registry.

15. CouncilOfDike can only finalize disputed cases.

16. Governance cannot bypass timelock for critical upgrades.

17. Admin pause cannot transfer user collateral.

18. LP shares must represent proportional pool ownership.

19. Fees must not be withdrawable from user collateral.

20. Cancelled markets must allow fair collateral recovery.
```

---

# 28. Testing Plan

## Unit Tests

Each module should have isolated unit tests.

### Factory Tests

```txt
Creates valid market
Rejects empty question
Rejects empty rules URI
Rejects past expiry
Rejects unsupported collateral
Rejects low liquidity
Rejects unauthorized creator
Registers market correctly
Initializes token IDs correctly
Initializes AMM pool correctly
```

### Registry Tests

```txt
Stores market data correctly
Allows valid status transitions
Rejects invalid status transitions
Rejects final outcome overwrite
Rejects unauthorized updates
Handles cancellation correctly
```

### Conditional Token Tests

```txt
Mints complete sets
Rejects mint without collateral
Merges YES/NO into collateral
Burns redeemed tokens
Prevents redemption before resolution
Handles YES final outcome
Handles NO final outcome
Handles INVALID refund
```

### Vault Tests

```txt
Receives collateral
Tracks market collateral
Releases collateral on merge
Pays YES winners
Pays NO winners
Pays invalid refunds
Prevents over-redemption
Prevents unauthorized withdrawal
Tracks redeemed amounts
```

### AMM Tests

```txt
Creates pool
Seeds liquidity
Calculates opening price
Buys YES
Buys NO
Sells YES
Sells NO
Updates reserves
Charges fees
Tracks LP shares
Adds liquidity
Removes liquidity
Rejects trades after expiry
Rejects trades after resolution
Enforces slippage
```

### CODOracle Tests

```txt
Requests resolution after expiry
Rejects request before expiry
Accepts proposal with bond
Rejects proposal without bond
Starts dispute window
Accepts dispute with matching bond
Rejects late dispute
Finalizes undisputed proposal
Escalates disputed case to council
Records final outcome from council
Rejects double finalization
```

### Council Tests

```txt
Opens disputed case
Allows council member commit
Rejects non-council vote
Allows valid reveal
Rejects invalid reveal
Tallies votes correctly
Handles YES majority
Handles NO majority
Handles INVALID majority
Finalizes after reveal phase
Rejects early finalization
Redistributes bonds
Rewards correct voters
```

---

# 29. Integration Tests

Integration tests should simulate full user flows.

## Flow 1: Basic YES Market

```txt
Create market
Seed liquidity
User buys YES
Market expires
Someone proposes YES
No dispute happens
Market resolves YES
User redeems YES
NO becomes worthless
```

## Flow 2: Basic NO Market

```txt
Create market
Seed liquidity
User buys NO
Market expires
Someone proposes NO
No dispute happens
Market resolves NO
User redeems NO
YES becomes worthless
```

## Flow 3: Disputed Market

```txt
Create market
Seed liquidity
User trades
Market expires
Proposer submits YES
Disputer submits NO
Case escalates to Council
Council votes NO
Market resolves NO
Disputer wins bond
Correct council voters receive rewards
NO holders redeem
```

## Flow 4: Invalid Market

```txt
Create market
Seed liquidity
Market expires
Proposer submits INVALID
No dispute happens
Market resolves INVALID
YES holders redeem 0.5
NO holders redeem 0.5
```

## Flow 5: Cancelled Market

```txt
Create market
Seed liquidity
Market is discovered to have bad rules
Governance cancels market
Trading stops
Users recover collateral
LPs withdraw proportional value
```

---

# 30. Fuzz Testing Targets

Fuzz tests should focus on financial invariants.

```txt
Random buy/sell sequences
Random LP deposits/withdrawals
Random splits/merges
Random expiry timings
Random proposal/dispute timings
Random redemption orders
Random fee configurations
Random reserve sizes
Random market liquidity
```

The key invariant to check:

```txt
Vault collateral should never be less than total valid redeemable claims.
```

---

# 31. Deployment Plan

## Phase 1: Local Development

Deploy:

```txt
Mock USDC
DikeMarketRegistry
DikeConditionalTokens
CollateralVault
FeeManager
DikeAMM
CODOracle
CouncilOfDike
DikeMarketFactory
```

Run:

```txt
Unit tests
Integration tests
Invariant tests
Gas snapshots
Manual lifecycle simulation
```

---

## Phase 2: Testnet Alpha

Enable:

```txt
Whitelisted market creation
Mock council members
USDC test token
Small liquidity markets
Short dispute windows
Manual resolution testing
```

Test:

```txt
Market creation
Trading
LP deposits
Resolution proposal
Disputes
Council voting
Redemption
Invalid market handling
Cancellation
Pause/unpause
```

---

## Phase 3: Public Testnet

Enable:

```txt
Public trading
Limited market creation by curators
More council members
Better frontend integration
Indexer integration
Keeper bots
Evidence upload support
```

Monitor:

```txt
Failed transactions
Oracle disputes
AMM reserve drift
LP withdrawals
Redemption correctness
Fee accounting
Council participation
```

---

## Phase 4: Mainnet Beta

Enable only:

```txt
Curated markets
Known collateral asset
Whitelisted council
Timelocked governance
Small market size limits
Conservative fees
Conservative bond requirements
Emergency pause
```

Do not enable:

```txt
Permissionless market creation
Native invalid token AMM
Token-weighted voting
Large uncapped markets
Cross-chain settlement
```

---

# 32. Recommended V1 Scope

The first version should include:

```txt
Binary YES/NO markets
USDC collateral
Curated market creation
ERC-1155-style outcome token model
Fixed-product AMM
LP liquidity
Trading fees
Market expiry
COD optimistic proposal
Dispute with matching bond
Whitelisted Council of Dike
Commit-reveal voting
Final outcome settlement
Winner redemption
Refund-style INVALID
Governance timelock
Emergency pause
```

Do not build in v1:

```txt
Orderbook
Permissionless market creation
Native INVALID outcome token
Token-weighted council voting
Advanced market templates
Cross-chain liquidity
Leverage
Borrowing
Prediction chaining
Dynamic oracle routing
```

---

# 33. Suggested Implementation Order

## Step 1: Market Registry

Build the market data model and state machine first.

Deliverables:

```txt
Market creation storage
Market status transitions
Final outcome storage
Access-controlled updates
```

## Step 2: Conditional Tokens

Build outcome token minting and burning.

Deliverables:

```txt
YES/NO token creation
Complete set minting
Complete set merging
Token redemption hooks
```

## Step 3: Collateral Vault

Build collateral custody and accounting.

Deliverables:

```txt
Deposit collateral
Track per-market collateral
Release collateral on merge
Redeem after resolution
Prevent over-redemption
```

## Step 4: Factory

Connect registry, tokens, vault, and AMM initialization.

Deliverables:

```txt
Create market
Validate parameters
Create token IDs
Initialize pool
Register market
```

## Step 5: AMM

Build trading and liquidity.

Deliverables:

```txt
Seed liquidity
Buy YES
Buy NO
Sell YES
Sell NO
Add liquidity
Remove liquidity
Track LP shares
Charge fees
```

## Step 6: CODOracle

Build optimistic resolution.

Deliverables:

```txt
Request resolution
Propose answer
Post bond
Open dispute window
Dispute answer
Finalize undisputed answer
Escalate disputed answer
```

## Step 7: CouncilOfDike

Build dispute voting.

Deliverables:

```txt
Open case
Commit vote
Reveal vote
Finalize vote
Redistribute bonds
Report final outcome
```

## Step 8: FeeManager

Centralize fee calculation and distribution.

Deliverables:

```txt
Trading fee calculation
LP/protocol/COD split
Proposal reward
Council reward
Treasury accounting
Governance-controlled updates
```

## Step 9: Governance + Timelock

Add admin safety.

Deliverables:

```txt
Parameter control
Council management
Fee management
Pause controls
Upgrade controls
Timelocked sensitive actions
```

## Step 10: Full System Testing

Run complete flows.

Deliverables:

```txt
YES resolution flow
NO resolution flow
Disputed flow
Invalid market flow
Cancelled market flow
LP flow
Fee flow
Emergency pause flow
```

---

# 34. Final Contract Dependency Graph

```txt
DikeMarketFactory
 |
 | writes market data
 v
DikeMarketRegistry
 |
 | creates outcome references
 v
DikeConditionalTokens
 |
 | backed by collateral
 v
CollateralVault

DikeAMM
 |
 | reads market status
 | uses outcome tokens
 | uses collateral vault
 v
FeeManager

CODOracle
 |
 | reads market rules
 | locks proposal/dispute bonds
 | updates market resolution state
 v
CouncilOfDike
 |
 | finalizes disputed cases
 v
CODOracle
 |
 | reports final answer
 v
DikeMarketRegistry
 |
 | enables redemption
 v
CollateralVault
```

---

# 35. Final Summary

The Dike smart contract system should be built as a modular protocol:

```txt
DikeMarketFactory:
Creates and initializes markets.

DikeMarketRegistry:
Stores market metadata, status, expiry, rules, and final outcome.

DikeConditionalTokens:
Creates and manages collateral-backed YES/NO outcome positions.

CollateralVault:
Holds USDC, tracks market collateral, and pays redemptions.

DikeAMM:
Handles trading, pricing, LP liquidity, and fees.

FeeManager:
Calculates and routes LP, protocol, and COD fees.

CODOracle:
Handles optimistic resolution, proposals, disputes, and finalization.

CouncilOfDike:
Acts as the dispute court for contested markets.

DikeGovernance + Timelock:
Controls upgrades, fees, council membership, and emergency permissions.
```

The simplest strong v1 is:

```txt
Curated binary markets
USDC collateral
YES/NO outcome tokens
AMM trading
LP liquidity
Optimistic proposal
Dispute bonds
Council of Dike voting
Final outcome settlement
Winner redemption
```

The core identity of the protocol:

> Dike lets users trade beliefs, LPs supply liquidity, and the Council of Dike settles truth when reality is disputed.
