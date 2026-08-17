#![cfg(test)]

use super::*;
use collateral_vault::{CollateralVault, CollateralVaultClient};
use conditional_tokens::{DikeConditionalTokens, DikeConditionalTokensClient};
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Env, String,
};

#[contracttype]
#[derive(Clone)]
pub enum RegKey {
    Collateral,
}

#[contract]
pub struct LiveRegistry;

#[contractimpl]
impl LiveRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn is_tradeable(_env: Env, _market_id: u64) -> Result<bool, DikeError> {
        Ok(true)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        Ok(test_market(&env, market_id, collateral, MarketStatus::Live))
    }
}

#[contract]
pub struct ClosedRegistry;

#[contractimpl]
impl ClosedRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn is_tradeable(_env: Env, _market_id: u64) -> Result<bool, DikeError> {
        Ok(false)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        Ok(test_market(
            &env,
            market_id,
            collateral,
            MarketStatus::TradingClosed,
        ))
    }
}

#[contract]
pub struct ResolvedRegistry;

#[contractimpl]
impl ResolvedRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn is_tradeable(_env: Env, _market_id: u64) -> Result<bool, DikeError> {
        Ok(false)
    }

    pub fn get_final_outcome(_env: Env, _market_id: u64) -> Result<Outcome, DikeError> {
        Ok(Outcome::Yes)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        let mut market = test_market(&env, market_id, collateral, MarketStatus::Resolved);
        market.has_final_outcome = true;
        market.final_outcome = Outcome::Yes;
        Ok(market)
    }
}

#[contract]
pub struct CancelledRegistry;

#[contractimpl]
impl CancelledRegistry {
    pub fn __constructor(env: Env, collateral: Address) {
        env.storage()
            .instance()
            .set(&RegKey::Collateral, &collateral);
    }

    pub fn is_tradeable(_env: Env, _market_id: u64) -> Result<bool, DikeError> {
        Ok(false)
    }

    pub fn get_market(env: Env, market_id: u64) -> Result<MarketData, DikeError> {
        let collateral: Address = env.storage().instance().get(&RegKey::Collateral).unwrap();
        Ok(test_market(
            &env,
            market_id,
            collateral,
            MarketStatus::Cancelled,
        ))
    }
}

fn test_market(env: &Env, market_id: u64, collateral: Address, status: MarketStatus) -> MarketData {
    MarketData {
        id: market_id,
        question: String::from_str(env, "q"),
        question_hash: soroban_sdk::BytesN::from_array(env, &[0; 32]),
        rules_uri: String::from_str(env, "rules"),
        rules_hash: soroban_sdk::BytesN::from_array(env, &[0; 32]),
        creator: Address::generate(env),
        collateral,
        yes_token_id: market_id * 2,
        no_token_id: market_id * 2 + 1,
        expiry: 1_000,
        status,
        has_final_outcome: false,
        final_outcome: Outcome::Invalid,
        pool_id: market_id,
        bond_amount: 1,
        dispute_window: 1,
        has_request: false,
        request_id: 0,
        created_at: env.ledger().timestamp(),
        fee_config: FeeConfig::default(),
    }
}

#[test]
fn seed_and_trade_updates_reserves() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(LiveRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &registry_id);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);
    let trader_start = TokenClient::new(&env, &token).balance(&trader);
    assert_eq!(
        client.try_buy_yes(&trader, &pool_id, &0, &1, &100),
        Err(Ok(DikeError::InvalidAmount))
    );
    assert_eq!(
        client.try_buy_no(&trader, &pool_id, &(-1), &1, &100),
        Err(Ok(DikeError::InvalidAmount))
    );
    assert_eq!(
        client.try_buy_child_yes(&trader, &1, &Outcome::Yes, &pool_id, &0, &1, &100),
        Err(Ok(DikeError::InvalidAmount))
    );
    assert_eq!(
        TokenClient::new(&env, &token).balance(&trader),
        trader_start
    );
    assert_eq!(vault.accounting(&1).collateral_backing, 10_000);

    let out = client.buy_yes(&trader, &pool_id, &1_000, &1, &100);
    assert!(out > 0);
    let pool = client.pool(&pool_id);
    assert!(pool.yes_reserve < 10_000);
    assert!(pool.no_reserve > 10_000);
    assert!(pool.accumulated_lp_fees > 0);
    assert_eq!(TokenClient::new(&env, &token).balance(&vault_id), 11_000);
    assert_eq!(vault.root_stake(&1, &trader, &Outcome::Yes), 1_000);
    assert_eq!(tokens.balance(&trader, &1, &Outcome::Yes), out);
    let acct = vault.accounting(&1);
    assert_eq!(acct.collateral_backing, 10_980);
    assert_eq!(acct.refundable, 10_980);
    assert_eq!(acct.lp_fees, 14);
    assert_eq!(acct.protocol_fees, 4);
    assert_eq!(acct.cod_fees, 2);
}

#[test]
fn child_buy_reuses_root_capital_without_extra_usdc() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &50_000);
    stellar.mint(&trader, &1_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(LiveRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &registry_id);

    let root_pool = client.create_pool(&1, &FeeConfig::default());
    let child_pool = client.create_pool(&2, &FeeConfig::default());
    client.seed_liquidity(&lp, &root_pool, &10_000);
    client.seed_liquidity(&lp, &child_pool, &10_000);

    let token_client = TokenClient::new(&env, &token);
    let starting_balance = token_client.balance(&trader);
    let root_out = client.buy_yes(&trader, &root_pool, &100, &1, &100);
    assert!(root_out > 0);
    assert_eq!(token_client.balance(&trader), starting_balance - 100);
    assert_eq!(
        vault.child_avail_for_outcome(&1, &trader, &Outcome::Yes),
        60
    );

    assert_eq!(
        client.try_buy_child_yes(&trader, &1, &Outcome::Yes, &child_pool, &61, &1, &100),
        Err(Ok(DikeError::ChildCollateralLimitExceeded))
    );

    let child_out = client.buy_child_yes(&trader, &1, &Outcome::Yes, &child_pool, &60, &1, &100);
    assert!(child_out > 0);
    assert_eq!(token_client.balance(&trader), starting_balance - 100);
    assert_eq!(vault.child_used_for_outcome(&1, &trader, &Outcome::Yes), 60);
    assert_eq!(
        vault.child_loan_for_outcome(&1, &Outcome::Yes, &2, &Outcome::Yes, &trader),
        60
    );
    assert_eq!(vault.parent_debt(&1, &trader, &Outcome::Yes), 60);
    assert_eq!(tokens.balance(&trader, &2, &Outcome::Yes), child_out);

    assert_eq!(
        client.try_buy_child_yes(&trader, &2, &Outcome::Yes, &root_pool, &1, &1, &100),
        Err(Ok(DikeError::ChainDepthExceeded))
    );
    assert_eq!(
        client.try_sell_yes(&trader, &child_pool, &1, &0, &100),
        Err(Ok(DikeError::EncumberedPosition))
    );
    assert_eq!(
        client.try_sell_yes(&trader, &root_pool, &1, &0, &100),
        Err(Ok(DikeError::EncumberedPosition))
    );
    assert_eq!(
        tokens.try_transfer_position(&trader, &Address::generate(&env), &2, &Outcome::Yes, &1),
        Err(Ok(DikeError::EncumberedPosition))
    );
    assert_eq!(
        tokens.try_transfer_position(&trader, &Address::generate(&env), &1, &Outcome::Yes, &1),
        Err(Ok(DikeError::EncumberedPosition))
    );
}

#[test]
fn full_amm_winner_redeems_with_registry_collateral_not_personal_deposit() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let wrong_token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let wrong_token = env
        .register_stellar_asset_contract_v2(wrong_token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let live_registry = env.register(LiveRegistry, (&token,));
    let resolved_registry = env.register(ResolvedRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &live_registry);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &live_registry);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);
    let out = client.buy_yes(&trader, &pool_id, &1_000, &1, &100);
    assert!(out > 1_000);
    vault.set_role(&symbol_short!("registry"), &resolved_registry);

    assert_eq!(
        vault.try_redeem_resolved(&wrong_token, &trader, &1, &Outcome::Yes, &out),
        Err(Ok(DikeError::UnsupportedCollateral))
    );

    let before = TokenClient::new(&env, &token).balance(&trader);
    let payout = vault.redeem_resolved(&token, &trader, &1, &Outcome::Yes, &out);
    assert_eq!(payout, out);
    assert_eq!(
        TokenClient::new(&env, &token).balance(&trader),
        before + out
    );
    assert_eq!(tokens.balance(&trader, &1, &Outcome::Yes), 0);
}

#[test]
fn sell_amm_position_is_not_capped_by_original_cash_stake() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(LiveRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &registry_id);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);
    let out = client.buy_yes(&trader, &pool_id, &1_000, &1, &100);
    assert!(out > 1_000);

    let payout = client.sell_yes(&trader, &pool_id, &out, &1, &100);
    assert!(payout > 0);
    assert_eq!(tokens.balance(&trader, &1, &Outcome::Yes), 0);
}

#[test]
fn registry_blocks_trading_when_market_is_not_live() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let live_registry = env.register(LiveRegistry, (&token,));
    let closed_registry = env.register(ClosedRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &live_registry);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &live_registry);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);
    client.set_modules(&vault_id, &tokens_id, &token, &closed_registry);
    vault.set_role(&symbol_short!("registry"), &closed_registry);

    assert_eq!(
        client.try_buy_yes(&trader, &pool_id, &1_000, &1, &100),
        Err(Ok(DikeError::InvalidStatus))
    );
    assert_eq!(
        client.try_sell_yes(&trader, &pool_id, &1, &1, &100),
        Err(Ok(DikeError::InvalidStatus))
    );
}

#[test]
fn skewed_pool_add_liquidity_prices_at_current_ratio_and_fees_are_claimable() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let lp2 = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&lp2, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let live_registry = env.register(LiveRegistry, (&token,));
    let resolved_registry = env.register(ResolvedRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &live_registry);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &live_registry);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);

    // Skew the pool hard by buying a large amount of YES.
    client.buy_yes(&trader, &pool_id, &5_000, &1, &100);
    let skewed = client.pool(&pool_id);
    assert!(skewed.yes_reserve < skewed.no_reserve);
    assert!(skewed.accumulated_lp_fees > 0);

    // Fair pricing: shares must be the min of both single-side ratios, not
    // just total_shares * amount / yes_reserve (which would over-mint here
    // since yes_reserve is now the smaller side).
    let naive_shares = skewed.total_lp_shares * 1_000 / skewed.yes_reserve;
    let fair_shares = skewed.total_lp_shares * 1_000 / skewed.no_reserve;
    assert!(fair_shares < naive_shares);
    let minted = client.add_liquidity(&lp2, &pool_id, &1_000);
    assert_eq!(minted, fair_shares);

    // lp2 joined after the first buy, so they must not be able to claim any
    // of the fees that were already accrued before they deposited.
    assert_eq!(client.claim_lp_fees(&lp2, &pool_id), 0);
    let lp_first_claim = client.claim_lp_fees(&lp, &pool_id);
    assert!(lp_first_claim > 0);
    assert_eq!(
        vault.accounting(&1).lp_fees,
        skewed.accumulated_lp_fees - lp_first_claim
    );

    // New trading activity after lp2 joined must be shared proportionally
    // between both LPs.
    client.buy_no(&trader, &pool_id, &2_000, &1, &100);
    let lp_second_claim = client.claim_lp_fees(&lp, &pool_id);
    let lp2_claim = client.claim_lp_fees(&lp2, &pool_id);
    assert!(lp_second_claim > 0);
    assert!(lp2_claim > 0);

    // Resolution no longer strands LP shares: remove_liquidity must work on
    // a resolved market and auto-claim any pending fees in the same call.
    vault.set_role(&symbol_short!("registry"), &resolved_registry);
    client.set_modules(&vault_id, &tokens_id, &token, &resolved_registry);
    let lp_balance_before = client.lp_balance(&pool_id, &lp);
    let (yes_out, no_out) = client.remove_liquidity(&lp, &pool_id, &lp_balance_before);
    assert!(yes_out > 0 && no_out > 0);
    assert_eq!(client.lp_balance(&pool_id, &lp), 0);

    let before = TokenClient::new(&env, &token).balance(&lp);
    let payout = vault.redeem_resolved(&token, &lp, &1, &Outcome::Yes, &yes_out);
    assert_eq!(payout, yes_out);
    assert_eq!(
        TokenClient::new(&env, &token).balance(&lp),
        before + yes_out
    );
    assert_eq!(tokens.balance(&lp, &1, &Outcome::No), no_out);
}

#[test]
fn cancelled_market_allows_lp_position_recovery_but_not_trading() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let live_registry = env.register(LiveRegistry, (&token,));
    let cancelled_registry = env.register(CancelledRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &live_registry);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &live_registry);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);
    client.set_modules(&vault_id, &tokens_id, &token, &cancelled_registry);
    vault.set_role(&symbol_short!("registry"), &cancelled_registry);

    assert_eq!(
        client.try_buy_yes(&trader, &pool_id, &100, &1, &100),
        Err(Ok(DikeError::InvalidStatus))
    );
    let (yes_out, no_out) = client.remove_liquidity(&lp, &pool_id, &10_000);
    assert_eq!(yes_out, 10_000);
    assert_eq!(no_out, 10_000);
    assert_eq!(tokens.balance(&lp, &1, &Outcome::Yes), 10_000);
    assert_eq!(tokens.balance(&lp, &1, &Outcome::No), 10_000);
}

// --- Item 2: unauthorized-caller negative-auth tests ---

#[test]
fn role_gated_fns_reject_unconfigured_role() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    // gov role not configured
    assert!(matches!(
        client.try_pause(&true),
        Err(Ok(DikeError::Unauthorized))
    ));
    // factory role not configured
    assert!(matches!(
        client.try_create_pool(&1, &FeeConfig::default()),
        Err(Ok(DikeError::Unauthorized))
    ));
}

#[test]
fn admin_gated_fns_reject_wrong_signer() {
    let env = Env::default(); // no mock_all_auths
    let admin = Address::generate(&env);
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    let other = Address::generate(&env);
    assert!(client.try_set_admin(&other).is_err());
    assert!(client.try_set_role(&symbol_short!("gov"), &other).is_err());
    assert!(client
        .try_set_modules(&other, &other, &other, &other)
        .is_err());
}

#[test]
fn sell_quote_average_price_matches_buy_side_convention() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    stellar.mint(&lp, &20_000);
    stellar.mint(&trader, &20_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let registry_id = env.register(LiveRegistry, (&token,));
    let id = env.register(DikeAMM, (&admin,));
    let client = DikeAMMClient::new(&env, &id);
    client.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    client.set_modules(&vault_id, &tokens_id, &token, &registry_id);

    let pool_id = client.create_pool(&1, &FeeConfig::default());
    client.seed_liquidity(&lp, &pool_id, &10_000);

    let tokens_out = client.buy_yes(&trader, &pool_id, &1_000, &1, &100);
    assert!(tokens_out > 0);

    let buy_quote = client.quote_buy_yes(&pool_id, &1_000);
    let sell_quote = client.quote_sell_yes(&pool_id, &tokens_out);

    // A single outcome token can never be worth more than 1 unit of
    // collateral, so average_price_bps (collateral-per-token, scaled by
    // 10_000) must never exceed 10_000. Previously the sell quote computed
    // tokens-per-collateral (the inverse), which routinely broke this bound.
    assert!(buy_quote.average_price_bps > 0 && buy_quote.average_price_bps <= 10_000);
    assert!(sell_quote.average_price_bps > 0 && sell_quote.average_price_bps <= 10_000);
}

fn setup_liquidation_env(
    env: &Env,
) -> (
    Address,
    CollateralVaultClient<'static>,
    DikeConditionalTokensClient<'static>,
    DikeAMMClient<'static>,
    Address,
) {
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(env);
    let factory = Address::generate(env);
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(env, &tokens_id);
    let registry_id = env.register(LiveRegistry, (&token,));
    let amm_id = env.register(DikeAMM, (&admin,));
    let amm = DikeAMMClient::new(env, &amm_id);
    amm.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &amm_id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &registry_id);
    tokens.set_role(&symbol_short!("amm"), &amm_id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    amm.set_modules(&vault_id, &tokens_id, &token, &registry_id);
    (token, vault, tokens, amm, registry_id)
}

#[test]
fn liquidate_position_rejects_above_threshold_and_succeeds_once_underwater() {
    let env = Env::default();
    let (token, vault, _tokens, amm, _registry_id) = setup_liquidation_env(&env);
    let stellar = StellarAssetClient::new(&env, &token);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let dumper = Address::generate(&env);
    let liquidator = Address::generate(&env);
    stellar.mint(&lp, &1_000_000);
    stellar.mint(&trader, &10_000);
    stellar.mint(&dumper, &1_000_000);

    let root_pool = amm.create_pool(&1, &FeeConfig::default());
    let child_pool = amm.create_pool(&2, &FeeConfig::default());
    amm.seed_liquidity(&lp, &root_pool, &500_000);
    amm.seed_liquidity(&lp, &child_pool, &500_000);

    // Trader buys YES, draws the full 60% child credit against it.
    amm.buy_yes(&trader, &root_pool, &1_000, &1, &100);
    let avail = vault.child_avail_for_outcome(&1, &trader, &Outcome::Yes);
    amm.buy_child_yes(&trader, &1, &Outcome::Yes, &child_pool, &avail, &1, &100);
    let debt = vault.child_used_for_outcome(&1, &trader, &Outcome::Yes);
    assert!(debt > 0);

    // Not liquidatable yet — trader's YES position is still worth close to
    // what they paid, comfortably above debt * 1.05.
    assert_eq!(
        amm.try_liquidate_position(&liquidator, &trader, &root_pool, &Outcome::Yes),
        Err(Ok(DikeError::NotLiquidatable))
    );

    // Crash the YES price hard: a huge NO buy from an unrelated dumper.
    amm.buy_no(&dumper, &root_pool, &400_000, &1, &100);

    let liquidator_balance_before = TokenClient::new(&env, &token).balance(&liquidator);
    let trader_balance_before = TokenClient::new(&env, &token).balance(&trader);

    let repaid = amm.liquidate_position(&liquidator, &trader, &root_pool, &Outcome::Yes);
    assert!(repaid > 0);
    assert!(repaid <= debt);

    // Keeper got paid, position was seized. Debt tracking always clears to
    // zero afterward — whatever proceeds couldn't repay goes through
    // resolve_parent_default (insurance/shortfall) rather than staying
    // marked as still-owed with no position left to liquidate against.
    assert!(TokenClient::new(&env, &token).balance(&liquidator) > liquidator_balance_before);
    assert_eq!(vault.child_used_for_outcome(&1, &trader, &Outcome::Yes), 0);
    assert_eq!(
        amm.try_liquidate_position(&liquidator, &trader, &root_pool, &Outcome::Yes),
        Err(Ok(DikeError::InvalidInput))
    );
    // Trader's own balance is untouched by the forced sale itself (they only
    // get whatever remainder is left after debt+bonus, which may be zero).
    assert!(TokenClient::new(&env, &token).balance(&trader) >= trader_balance_before);
}

#[test]
fn liquidate_position_blocked_while_market_not_live() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1);
    let admin = Address::generate(&env);
    let factory = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar = StellarAssetClient::new(&env, &token);
    let lp = Address::generate(&env);
    let trader = Address::generate(&env);
    let liquidator = Address::generate(&env);
    stellar.mint(&lp, &1_000_000);
    stellar.mint(&trader, &10_000);

    let vault_id = env.register(CollateralVault, (&admin, &admin));
    let vault = CollateralVaultClient::new(&env, &vault_id);
    let tokens_id = env.register(DikeConditionalTokens, (&admin,));
    let tokens = DikeConditionalTokensClient::new(&env, &tokens_id);
    let live_registry = env.register(LiveRegistry, (&token,));
    let closed_registry = env.register(ClosedRegistry, (&token,));
    let amm_id = env.register(DikeAMM, (&admin,));
    let amm = DikeAMMClient::new(&env, &amm_id);
    amm.set_role(&symbol_short!("factory"), &factory);
    vault.set_role(&symbol_short!("amm"), &amm_id);
    vault.set_role(&symbol_short!("tokens"), &tokens_id);
    vault.set_role(&symbol_short!("registry"), &live_registry);
    tokens.set_role(&symbol_short!("amm"), &amm_id);
    tokens.set_role(&symbol_short!("vault"), &vault_id);
    amm.set_modules(&vault_id, &tokens_id, &token, &live_registry);

    let root_pool = amm.create_pool(&1, &FeeConfig::default());
    let child_pool = amm.create_pool(&2, &FeeConfig::default());
    amm.seed_liquidity(&lp, &root_pool, &500_000);
    amm.seed_liquidity(&lp, &child_pool, &500_000);
    amm.buy_yes(&trader, &root_pool, &1_000, &1, &100);

    // Registry flips to a non-Live status (stands in for
    // ResolutionRequested/Disputed/CouncilVoting — all map to
    // is_tradeable == false the same way): liquidation must be blocked, a
    // disputed outcome could still flip on appeal.
    vault.set_role(&symbol_short!("registry"), &closed_registry);
    amm.set_modules(&vault_id, &tokens_id, &token, &closed_registry);

    assert_eq!(
        amm.try_liquidate_position(&liquidator, &trader, &root_pool, &Outcome::Yes),
        Err(Ok(DikeError::InvalidStatus))
    );
}
