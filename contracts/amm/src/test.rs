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
