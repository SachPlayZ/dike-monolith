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

pub fn collateral_limit(amount: i128, collateral_bps: u32) -> Result<i128, DikeError> {
    if amount < 0 {
        return Err(DikeError::InvalidAmount);
    }
    bps(amount, collateral_bps)
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

pub fn quote_buy_complete_set(
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
    checked_sub(checked_add(outcome_reserve, net_in)?, new_outcome)
}

// Largest integer x such that x*x <= n (Babylonian method).
fn isqrt(n: i128) -> Result<i128, DikeError> {
    if n < 0 {
        return Err(DikeError::InvalidAmount);
    }
    if n < 2 {
        return Ok(n);
    }
    let mut x = n;
    let mut y = checked_div(checked_add(x, 1)?, 2)?;
    while y < x {
        x = y;
        y = checked_div(checked_add(x, checked_div(n, x)?)?, 2)?;
    }
    Ok(x)
}

// Selling amount_in of the outcome side must return collateral_out such that redeeming it
// (mirroring quote_buy_complete_set's mint step) preserves the pool's constant product:
//   (outcome_reserve + amount_in - collateral_out) * (opposite_reserve - collateral_out)
//     == outcome_reserve * opposite_reserve
// This is quadratic in collateral_out: c^2 - c*(O+S+P) + S*P = 0, solved for the smaller root.
// (A naive single-division "swap" formula — treating this like a plain two-asset CPMM — is
// wrong here: it ignores that both reserves are backed by the same shared collateral and pays
// out roughly 1 unit of collateral per token instead of ~price-per-token.)
pub fn quote_sell(
    outcome_reserve: i128,
    opposite_reserve: i128,
    amount_in: i128,
) -> Result<i128, DikeError> {
    if outcome_reserve <= 0 || opposite_reserve <= 0 || amount_in <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let sum = checked_add(checked_add(outcome_reserve, amount_in)?, opposite_reserve)?;
    let sum_sq = checked_mul(sum, sum)?;
    let four_sp = checked_mul(4, checked_mul(amount_in, opposite_reserve)?)?;
    let discriminant = checked_sub(sum_sq, four_sp)?;
    if discriminant < 0 {
        return Err(DikeError::ArithmeticError);
    }
    let sqrt_disc = isqrt(discriminant)?;
    checked_div(checked_sub(sum, sqrt_disc)?, 2)
}

pub fn invalid_refund(amount: i128) -> Result<i128, DikeError> {
    checked_div(amount, 2)
}

pub fn average_price_bps(amount_in: i128, amount_out: i128) -> Result<u32, DikeError> {
    if amount_in <= 0 || amount_out <= 0 {
        return Err(DikeError::InvalidAmount);
    }
    let price = checked_div(checked_mul(amount_in, BPS_DENOMINATOR)?, amount_out)?;
    if price > u32::MAX as i128 {
        return Err(DikeError::ArithmeticError);
    }
    Ok(price as u32)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn average_price_rejects_u32_overflow() {
        assert_eq!(
            average_price_bps((u32::MAX as i128) + 1, 1),
            Err(DikeError::ArithmeticError)
        );
    }

    #[test]
    fn quote_sell_matches_fpmm_price_at_balanced_pool() {
        // 100/100 USDC pool (50/50 price, 7-decimal fixed point), selling 1 token should
        // return ~0.49875 USDC, not ~0.99 (a naive single-swap formula, matching a plain
        // 2-asset CPMM, would wrongly return ~0.99 here — that's the arbitrage bug this
        // test guards against).
        let out = quote_sell(1_000_000_000, 1_000_000_000, 10_000_000).unwrap();
        assert_eq!(out, 4_987_500);
    }

    #[test]
    fn quote_sell_is_inverse_of_quote_buy_complete_set_up_to_rounding() {
        // Buying then immediately selling the same output should not yield a profit —
        // round-tripping loses a little to integer rounding, never gains.
        let (yes, no) = (500_000_000, 500_000_000);
        let bought = quote_buy_complete_set(yes, no, 10_000_000).unwrap();
        let new_yes = yes + 10_000_000 - bought;
        let new_no = no + 10_000_000;
        let sold_back = quote_sell(new_yes, new_no, bought).unwrap();
        assert!(
            sold_back <= 10_000_000,
            "round-trip must not profit: put in 10_000_000, got back {sold_back}"
        );
    }

    #[test]
    fn isqrt_matches_known_values() {
        assert_eq!(isqrt(0).unwrap(), 0);
        assert_eq!(isqrt(1).unwrap(), 1);
        assert_eq!(isqrt(4).unwrap(), 2);
        assert_eq!(isqrt(15).unwrap(), 3);
        assert_eq!(isqrt(16).unwrap(), 4);
        assert_eq!(isqrt(1_000_000).unwrap(), 1_000);
        assert_eq!(isqrt(-1), Err(DikeError::InvalidAmount));
    }
}
