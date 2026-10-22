//! Paying for items from the spending money and taking their price back,
//! as the original's Equipment page does when it buys and sells.

use crate::lists::COINS;

/// Copper, silver, electrum, gold and platinum pieces, as
/// [`CharacterLists::coins`](crate::lists::CharacterLists::coins) holds
/// them.
pub type Coins = [u32; 5];

const GOLD: usize = 3;
const SILVER: usize = 1;
const ELECTRUM: usize = 2;

/// What the coins are worth in copper pieces (5e38e6).
pub fn worth(coins: &Coins) -> u64 {
    coins
        .iter()
        .zip(COINS)
        .map(|(&n, (_, value))| u64::from(n) * u64::from(value))
        .sum()
}

/// Pays `amount` copper pieces from the coins, or pays nothing and gives
/// false when they are not worth it.
///
/// As the original's 5dde7f does, it pays with copper when the copper
/// covers it, otherwise with the cheapest kind of coin that, with the
/// coins below it, covers it, and gives the change in that coin, gold,
/// silver and copper. The original weighs each kind of coin alone, so it
/// may pay nothing at all, and it leaves the electrum it spent as it was;
/// here the coins below count and nothing is made out of nothing.
pub fn pay(coins: &mut Coins, amount: u32) -> bool {
    let amount = u64::from(amount);
    for top in 0..coins.len() {
        let pool: u64 = coins[..=top]
            .iter()
            .zip(COINS)
            .map(|(&n, (_, value))| u64::from(n) * u64::from(value))
            .sum();
        if pool < amount {
            continue;
        }
        let mut left = pool - amount;
        for c in (0..=top).rev() {
            let value = u64::from(COINS[c].1);
            if c == top || c != ELECTRUM {
                let n = left / value;
                coins[c] = u32::try_from(n).unwrap_or(u32::MAX);
                left -= n * value;
            } else {
                coins[c] = 0;
            }
        }
        return true;
    }
    false
}

/// Gives `amount` copper pieces back as gold, silver and copper, which are
/// then counted out again in as few coins as can be (5ddd19).
pub fn refund(coins: &mut Coins, amount: u32) {
    let total = u64::from(coins[GOLD]) * 100
        + u64::from(coins[SILVER]) * 10
        + u64::from(coins[0])
        + u64::from(amount);
    coins[GOLD] = u32::try_from(total / 100).unwrap_or(u32::MAX);
    coins[SILVER] = (total % 100 / 10) as u32;
    coins[0] = (total % 10) as u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paying_takes_the_amount_and_keeps_the_rest() {
        let start = [7, 6, 3, 14, 2];
        for amount in [0, 5, 7, 8, 60, 67, 150, 1399, 2500] {
            let mut coins = start;
            assert!(pay(&mut coins, amount));
            assert_eq!(worth(&coins), worth(&start) - u64::from(amount));
        }
    }

    #[test]
    fn paying_too_much_pays_nothing() {
        let start = [7, 6, 3, 14, 2];
        let mut coins = start;
        assert!(!pay(&mut coins, worth(&start) as u32 + 1));
        assert_eq!(coins, start);
    }

    #[test]
    fn paying_uses_the_cheapest_coins_that_cover_it() {
        let mut coins = [7, 6, 3, 14, 2];
        assert!(pay(&mut coins, 5));
        assert_eq!(coins[1..], [6, 3, 14, 2]);
        assert!(pay(&mut coins, 30));
        assert_eq!(coins[2..], [3, 14, 2]);
    }

    #[test]
    fn a_refund_is_counted_out_in_gold_silver_and_copper() {
        let start = [7, 6, 3, 14, 2];
        let mut coins = start;
        refund(&mut coins, 145);
        assert_eq!(worth(&coins), worth(&start) + 145);
        assert!(coins[0] < 10 && coins[1] < 10);
        assert_eq!((coins[2], coins[4]), (start[2], start[4]));
    }
}
