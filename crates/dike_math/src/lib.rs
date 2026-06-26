#![no_std]

use dike_types::{DikeError, BPS_DENOMINATOR};

pub fn checked_add(a: i128, b: i128) -> Result<i128, DikeError> {
    a.checked_add(b).ok_or(DikeError::ArithmeticError)
}

pub fn checked_sub(a: i128, b: i128) -> Result<i128, DikeError> {
    a.checked_sub(b).ok_or(DikeError::ArithmeticError)
}

pub fn checked_mul(a: i128, b: i128) -> Result<i128, DikeError> {
    a.checked_mul(b).ok_or(DikeError::ArithmeticError)
}

pub fn checked_div(a: i128, b: i128) -> Result<i128, DikeError> {
    if b == 0 {
        return Err(DikeError::InvalidAmount);
    }
    a.checked_div(b).ok_or(DikeError::ArithmeticError)
}

pub fn bps(amount: i128, rate_bps: u32) -> Result<i128, DikeError> {
    checked_div(checked_mul(amount, rate_bps as i128)?, BPS_DENOMINATOR)
}

pub fn required_bond(
    minimum_bond: i128,
    market_liquidity: i128,
    bond_bps: u32,
) -> Result<i128, DikeError> {
    let scaled = bps(market_liquidity, bond_bps)?;
    if scaled > minimum_bond {
        Ok(scaled)
    } else {
        Ok(minimum_bond)
    }
}

pub fn split_fee(amount: i128, fee_bps: u32) -> Result<(i128, i128), DikeError> {
    let fee = bps(amount, fee_bps)?;
    Ok((fee, checked_sub(amount, fee)?))
}

pub fn proportional(amount: i128, shares: i128, total_shares: i128) -> Result<i128, DikeError> {
    checked_div(checked_mul(amount, shares)?, total_shares)
}

pub fn quote_buy(
    outcome_reserve: i128,
    opposite_reserve: i128,
    net_in: i128,
) -> Result<i128, DikeError> {
    if outcome_reserve <= 0 || opposite_reserve <= 0 || net_in <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let k = checked_mul(outcome_reserve, opposite_reserve)?;
    let new_opposite = checked_add(opposite_reserve, net_in)?;
    let new_outcome = checked_div(k, new_opposite)?;
    checked_sub(outcome_reserve, new_outcome)
}

pub fn quote_sell(
    outcome_reserve: i128,
    opposite_reserve: i128,
    amount_in: i128,
) -> Result<i128, DikeError> {
    if outcome_reserve <= 0 || opposite_reserve <= 0 || amount_in <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let k = checked_mul(outcome_reserve, opposite_reserve)?;
    let new_outcome = checked_add(outcome_reserve, amount_in)?;
    let new_opposite = checked_div(k, new_outcome)?;
    checked_sub(opposite_reserve, new_opposite)
}

pub fn invalid_refund(amount: i128) -> Result<i128, DikeError> {
    checked_div(amount, 2)
}

pub fn average_price_bps(amount_in: i128, amount_out: i128) -> Result<u32, DikeError> {
    if amount_in <= 0 || amount_out <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let price = checked_div(checked_mul(amount_in, BPS_DENOMINATOR)?, amount_out)?;
    Ok(price as u32)
}
