#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env};

#[test]
fn accounting_starts_empty() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let id = env.register(CollateralVault, (&admin, &treasury));
    let client = CollateralVaultClient::new(&env, &id);

    let acct = client.accounting(&1);
    assert_eq!(acct.total_deposited, 0);
    assert_eq!(acct.collateral_backing, 0);
}
