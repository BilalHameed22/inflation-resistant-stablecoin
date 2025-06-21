#![allow(unexpected_cfgs)]
// #![feature(trivial_bounds)]
// #[cfg(feature = "idl-build")]

use anchor_lang::prelude::*;
use anchor_lang::*;

use crate::Stablecoins::*;

// The number of stablecoins that are currently supported by the IRMA program.
pub const BACKING_COUNT: usize = Stablecoins::USDE as usize;

declare_id!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");

/// IRMA module
/// FIXME: the decimals are all assumed to be zero, which is not true for all stablecoins.

// All currently existing stablecoins with about $100 M in circulation
// are supported. This list is not exhaustive and will be updated as new
// stablecoins are added to the market.
// Initially, we will support only those stablecoins that exist
// on the Solana blockchain (the first six below). 
#[derive(Debug, AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Stablecoins {
    USDT, // <== from Tether, $2.39 B in circulation
    USDC, // <== from Circle, $8.9 B in circulation
    USDS, // <== from Sky (previously MakerDAO) #19, $82. M in circulation
    PYUSD, // <== from PayPal #98, $224 M in circulation
    USDG, // <== from Singapore #263, $96 M in circulation
    FDUSD, // <== First Digital USD, $104 M in circulation
    USDE, // from Ethena #31, $9.9 M in circulation
    USDP, // from Paxos #551, $1.66 M in circulation
    SUSD, // from Solayer, has 4 to 5% yield #839, $13.9 M in circulation
    ZUSD, // from GMO-Z #1165, $8.9 M in circulation
    USDR, // from StabIR #1884, does not exist on Solana yet
    DAI,  // thru Wormhole, very low liquidity in Solana
    USD1,
    EnumCount
}

impl Stablecoins {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Stablecoins::USDT,
            1 => Stablecoins::USDC,
            2 => Stablecoins::USDS,
            3 => Stablecoins::PYUSD,
            4 => Stablecoins::USDG,
            5 => Stablecoins::FDUSD,
            6 => Stablecoins::USDE,
            7 => Stablecoins::USDP,
            8 => Stablecoins::SUSD,
            9 => Stablecoins::ZUSD,
            10 => Stablecoins::USDR,
            11 => Stablecoins::DAI,
            12 => Stablecoins::USD1,
            _ => Stablecoins::EnumCount,
        }
    }

    /// Converts the Stablecoins enum to an index.
    /// This is not needed because the index is just "Stablecoins::whatever as usize"
    pub fn to_index(&self) -> usize {
        self.clone() as usize
    }

    pub fn to_string(&self) -> String {
        match self {
            Stablecoins::USDT => "USDT".to_string(),
            Stablecoins::USDC => "USDC".to_string(),
            Stablecoins::USDS => "USDS".to_string(),
            Stablecoins::PYUSD => "PYUSD".to_string(),
            Stablecoins::USDG => "USDG".to_string(),
            Stablecoins::FDUSD => "FDUSD".to_string(),
            Stablecoins::USDE => "USDE".to_string(),
            Stablecoins::USDP => "USDP".to_string(),
            Stablecoins::SUSD => "SUSD".to_string(),
            Stablecoins::ZUSD => "ZUSD".to_string(),
            Stablecoins::USDR => "USDR".to_string(),
            Stablecoins::DAI => "DAI".to_string(),
            Stablecoins::USD1 => "USD1".to_string(),
            Stablecoins::EnumCount => "EnumCount".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "USDT" => Stablecoins::USDT,
            "USDC" => Stablecoins::USDC,
            "USDS" => Stablecoins::USDS,
            "PYUSD" => Stablecoins::PYUSD,
            "USDG" => Stablecoins::USDG,
            "FDUSD" => Stablecoins::FDUSD,
            "USDE" => Stablecoins::USDE,
            "USDP" => Stablecoins::USDP,
            "SUSD" => Stablecoins::SUSD,
            "ZUSD" => Stablecoins::ZUSD,
            "USDR" => Stablecoins::USDR,
            "DAI" => Stablecoins::DAI,
            "USD1" => Stablecoins::USD1,
            _ => Stablecoins::EnumCount,
        }
    }
}


pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    msg!("Greetings from: {:?}", ctx.program_id);
    let state = &mut ctx.accounts.state;
    if state.mint_price.len() > 0 {
        return Ok(());
    }
    state.mint_price = Vec::<f64>::with_capacity(EnumCount as usize);
    msg!("Vec capacity: {:?}", state.mint_price.capacity());
    state.backing_reserves = Vec::<u64>::with_capacity(EnumCount as usize);
    state.irma_in_circulation = Vec::<u64>::with_capacity(EnumCount as usize);
    state.backing_decimals = Vec::<u8>::with_capacity(EnumCount as usize);
    state.mint_price = vec![1.0; BACKING_COUNT];
    msg!("Vec length: {:?}", state.mint_price.len());
    state.irma_in_circulation = vec![1; BACKING_COUNT];
    state.backing_reserves = vec![0; BACKING_COUNT];
    // USDR and USD1 are not yet in Solana, so we set their decimals to 
    // the following are also set to 0 (disabled): USDE, USDP, SUSD, ZUSD, and DAI.
    state.backing_decimals = vec![6, 6, 6, 6, 6, 6, 0, 0, 0, 0, 0, 0, 0];
    state.bump = 13u8; // Bump seed for the PDA
    Ok(())
}

pub fn hello(ctx: Context<SetMintPrice>) -> Result<()> {
    let state = &mut ctx.accounts.state;
    if state.mint_price.len() == 0 {
        state.mint_price = vec![1.0; BACKING_COUNT];
        state.backing_reserves = vec![0; BACKING_COUNT];
        state.irma_in_circulation = vec![0; BACKING_COUNT];
    }
    msg!("State initialized with mint prices: {:?}", state.mint_price);
    msg!("Backing reserves: {:?}", state.backing_reserves);
    msg!("Irma in circulation: {:?}", state.irma_in_circulation);
    msg!("Program ID: {:?}", ctx.program_id);
    msg!("Hello world...");
    Ok(())
}

/// SetMintPrice of IRMA expressed in terms of a given quote token.
/// This should be called for every backing stablecoin supported, only once per day
/// because Truflation updates the inflation data only once per day.
pub fn set_mint_price(ctx: Context<SetMintPrice>, quote_token: Stablecoins, mint_price: f64) -> Result<()> {
    let state = &mut ctx.accounts.state;
    require!(state.backing_decimals[quote_token as usize] > 0, CustomError::InvalidQuoteToken);
    
    let curr_price = state.mint_price.get_mut(quote_token as usize).unwrap();
    require!(mint_price > 0.0, CustomError::InvalidAmount);
    *curr_price = mint_price;
    Ok(())
}

/// Mint IRMA tokens for a given amount of quote token.
/// FIXME: Currently assumes that decimal point is zero digits for both IRMA and quote token.
pub fn mint_irma(ctx: Context<MintIrma>, quote_token: Stablecoins, amount: u64) -> Result<()> {
    require!(amount > 0, CustomError::InvalidAmount);

    let state: &mut Account<'_, State> = &mut ctx.accounts.state;
    require!(state.backing_decimals[quote_token as usize] > 0, CustomError::InvalidQuoteToken);

    let backing_reserve: &mut u64 = state.backing_reserves.get_mut(quote_token as usize).unwrap();
    // require!(*backing_reserve > 0, CustomError::InsufficientReserve);
    *backing_reserve += amount;

    let curr_price: &mut f64 = state.mint_price.get_mut(quote_token as usize).unwrap();
    require!(*curr_price > 0.0, CustomError::MintPriceNotSet);

    let price: f64 = (*curr_price).clone();

    let circulation: &mut u64 = state.irma_in_circulation.get_mut(quote_token as usize).unwrap();
    require!(*circulation > 0, CustomError::InsufficientCirculation);

    *circulation += (amount as f64 / price).ceil() as u64;

    Ok(())
}

/// RedeemIRMA - user surrenders IRMA in irma_amount, expecting to get back quote_token according to redemption price.
/// FIXME: If resulting redemption price increases by more than 0.0000001, then actual redemption price 
/// should be updated immediately.
pub fn redeem_irma(ctx: Context<RedeemIrma>, quote_token: Stablecoins, irma_amount: u64) -> Result<()> {
    let state = &mut ctx.accounts.state;
    require!(state.backing_decimals[quote_token as usize] > 0, CustomError::InvalidQuoteToken);

    if irma_amount == 0 { return Ok(()) };

    // There is a redemption rule: every redemption is limited to 100k IRMA or 10% of the IRMA in circulation (for
    // the quote token) whichever is smaller.
    let circulation: u64 = state.irma_in_circulation[quote_token as usize];
    require!((irma_amount <= 100_000) && (irma_amount <= circulation / 10), CustomError::InvalidIrmaAmount);

    state.reduce_circulations(quote_token, irma_amount)?;

    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, space=26*BACKING_COUNT, payer=irma_admin, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub irma_admin: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetMintPrice<'info> {
    #[account(mut, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintIrma<'info> {
    #[account(mut, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RedeemIrma<'info> {
    #[account(mut, seeds=[b"state".as_ref()], bump)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub trader: Signer<'info>,
    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
#[derive(Debug)]
pub struct State {
    #[max_len(BACKING_COUNT)]
    pub mint_price: Vec<f64>,
    #[max_len(BACKING_COUNT)]
    pub backing_reserves: Vec<u64>,
    #[max_len(BACKING_COUNT)]
    pub backing_decimals: Vec<u8>,
    #[max_len(BACKING_COUNT)]
    pub irma_in_circulation: Vec<u64>,
    pub bump: u8,
}


/// ReduceCirculations implementation
/// This now deals with mint_price being less than redemption_price (a period of deflation).
/// If the price of the underlying reserve goes up with respect to USD, its exchange rate with IRMA
/// would improve (i.e. IRMA would be worth less in terms of the reserve). In this case, the system
/// would be expected to have a higher redemption price for IRMA than mint price; however, because
/// the objective is always to preserve the backing, the system will not allow the mint price 
/// to be less than the redemption price. Instead, it will simply set the redemption price to the mint price.
impl State {

    fn reduce_circulations(&mut self, quote_token: Stablecoins, irma_amount: u64) -> Result<()> {
        require!(irma_amount > 0, CustomError::InvalidAmount);
        require!((quote_token as usize) < BACKING_COUNT, CustomError::InvalidQuoteToken);
        require!(self.mint_price.len() > 0, CustomError::MintPriceNotSet);
        require!(self.backing_reserves.len() > 0, CustomError::InsufficientReserve);
        require!(self.irma_in_circulation.len() > 0, CustomError::InsufficientCirculation);
        // determine what this redemption does:
        // does it keep the relative spreads even, or does it skew the spreads?
        let mut count: u8 = 0;
        let mut average_diff: f64 = 0.0;
        let price_differences : Vec<f64> = self.backing_reserves.iter()
            .enumerate()
            .filter_map(|(i, reserve)| {
                let circulation = self.irma_in_circulation[i];
                let redemption_price = *reserve as f64 / circulation as f64;
                let mint_price = self.mint_price[i];
                if mint_price == 0.0 || self.backing_decimals[i] == 0 {
                    // msg!("Skipping {}: mint_price is 0.0 or backing_decimals is 0", Stablecoins::from_index(i).unwrap().to_string());
                    return Some(0.0);
                }
                count += 1;
                let x: f64 = mint_price - redemption_price;
                average_diff += x;
                Some(x)
            })
            .collect();
        if count == 0 {
            // msg!("No price differences found, returning early.");
            return Ok(());
        }
        average_diff /= count as f64;
        // msg!("Average price difference: {}", average_diff);

        let min_diff: f64 = 0.001; // price differences below this are ignored

        let mut max_price_diff: f64 = average_diff;
        let mut other_target: Stablecoins = quote_token;
        for (i, price_diff) in price_differences.iter().enumerate() {
            // msg!("{}: {}, max {}", i, *price_diff, max_price_diff);
            if (*price_diff - max_price_diff).abs() > min_diff && *price_diff > max_price_diff {
                max_price_diff = *price_diff;
                other_target = Stablecoins::from_index(i);
            }
        }
        // msg!("Max token: {}", other_target.to_string());
        // msg!("Max price diff: {}", max_price_diff);

        let ro_circulation: u64 = self.irma_in_circulation[quote_token as usize];
        let reserve: &mut u64 = self.backing_reserves.get_mut(quote_token as usize).unwrap();
        let redemption_price: f64 = *reserve as f64 / ro_circulation as f64;
        let subject_adjustment: u64 = (irma_amount as f64 * redemption_price).ceil() as u64;

        // no matter what, we need to reduce the subject reserve (quote_token)
        require!(*reserve >= subject_adjustment, CustomError::InsufficientReserve);
        *reserve -= subject_adjustment;

        // if max price diff does not deviate much from average diff or all inflation-adjusted prices 
        // are less than the redemption prices, then reductions pertain to quote_token only.
        if (average_diff.abs() < min_diff) || (average_diff < 0.0) {
            // msg!("No significant price differences found");
            if price_differences[quote_token as usize] >= 0.0 || other_target == quote_token {
                // msg!("If quote_token m price is larger than r price, then situation is normal.");
                // If the price difference is positive, it means that the mint price is higher than the redemption price;
                // in this case, we need to reduce IRMA in circulation by the irma_amount.
                // Note that this keeps price differences the same (it's minting that adjusts redemption price).
                let circulation: &mut u64 = self.irma_in_circulation.get_mut(quote_token as usize).unwrap();
                require!(*circulation >= irma_amount, CustomError::InsufficientCirculation);
                *circulation -= irma_amount;
            } else {
                msg!("m price <= r price for quote token, adjust backing reserve only for {}.", quote_token.to_string());
                // If the price difference is negative, it means that the mint price is lower than the redemption price;
                // in this case, we need to set the redemption price eq to the mint price in order to preserve the backing.
                // We also do not reduce IRMA in circulation, which effectively means that we are still draining the reserve,
                // but not by much, while the reduction in the ratio of reserve to IRMA in circulation (normally the
                // redemption price) goes down faster than if we also reduced IRMA in circulation. 
                // And we're done!
            }
            // msg!("New reserve for {}: {}", quote_token.to_string(), *reserve);
            // let ro_circulation: u64 = self.irma_in_circulation[quote_token as usize];
            // msg!("New circulation for {}: {}", quote_token.to_string(), ro_circulation);
            return Ok(());
        }
        // All the following code is for the semi-normal case, in which the mint price 
        // is higher than or equal to the redemption price; but the price differences
        // can be large.
        // msg!("Other target for normal adjustments: {}", other_target.to_string());

        let other_circulation: u64 = self.irma_in_circulation[other_target as usize];

        // if we don't have enough reserve to redeem the irma_amount, just error out;
        // we can't allow redemption from a reserve that is smaller than the irma_amount.
        // require!(irma_amount <= *circulation, CustomError::InsufficientCirculation);

        let other_price: f64 = self.mint_price[other_target as usize];
        let price: f64 = self.mint_price[quote_token as usize];
        let other_reserve: u64 = self.backing_reserves[other_target as usize];
        let reserve: u64 = self.backing_reserves[quote_token as usize];

        let other_price_diff: f64 = other_price - (other_reserve / other_circulation) as f64;
        let ro_circulation: u64 = self.irma_in_circulation[quote_token as usize];
        let post_price_diff: f64 = price - (reserve as f64 - irma_amount as f64 / price) / ro_circulation as f64;
        let post_other_price_diff: f64 = other_price - (other_reserve as f64 / (other_circulation - irma_amount) as f64);

        if other_price_diff <= post_other_price_diff {
            // msg!("--> Other price diff is less than or equal to post other price diff, adjusting second circulation only.");
            // if irma_amount is such that it could not improve the redemption price when applied to other stabecoin reserve,
            // we can just subtract from the circulation (same as normal case).
            // Note that the normal case does not change redemtion prices.
            let circulation: &mut u64 = self.irma_in_circulation.get_mut(quote_token as usize).unwrap();
            require!(irma_amount <= *circulation, CustomError::InsufficientCirculation);
            *circulation -= irma_amount;
        } else
        if post_other_price_diff <= post_price_diff {
            // msg!("--> Post other price diff is less than or equal to second price diff, 
            //         adjusting other circulation only.");
            // if irma_amount is such that it would reduce discrepancy for other stablecoin more post 
            // adjustment, we can choose to subtract irma_amount from the other_circulation only
            require!(irma_amount <= other_circulation, CustomError::InsufficientCirculation);
            let other_circulation = self.irma_in_circulation.get_mut(other_target as usize).unwrap();
            *other_circulation -= irma_amount;
        } else {
            // if irma amount is such that it doesn't improve the redemption price for either stablecoin,
            // we can do a linear adjustment of both other and second circulations.
            // msg!("--> First and second prices are close enough, adjusting both circulations linearly.");
            // Do simple linear adjustment of both other and second circulations
            let adjustment_amount: f64 = irma_amount as f64 * (other_price_diff - post_price_diff) / (other_price_diff + post_price_diff);
            // msg!("Adjustment amount: {}", adjustment_amount);
            require!(adjustment_amount > 0.0, CustomError::InvalidAmount);
            require!(adjustment_amount <= irma_amount as f64, CustomError::InvalidAmount);
            // msg!("Adjusting other circulation by {} and second circulation by {}", adjustment_amount.ceil(), irma_amount as f64 - adjustment_amount.ceil());
            let other_circulation: &mut u64 = self.irma_in_circulation.get_mut(other_target as usize).unwrap();
            *other_circulation -= adjustment_amount.ceil() as u64;
            let circulation: &mut u64 = self.irma_in_circulation.get_mut(quote_token as usize).unwrap();
            *circulation -= irma_amount - adjustment_amount.ceil() as u64;
        }

        return Ok(());
    }
}


#[error_code]
pub enum CustomError {
    #[msg("Invalid amount provided.")]
    InvalidAmount,
    #[msg("Mint price not set.")]
    MintPriceNotSet,
    #[msg("Invalid quote token.")]
    InvalidQuoteToken,
    #[msg("Insufficient circulation.")]
    InsufficientCirculation,
    #[msg("Insufficient reserve.")]
    InsufficientReserve,
    #[msg("Invalid reserve value.")]
    InvalidBacking,
    #[msg("Invalid IRMA amount.")]
    InvalidIrmaAmount,
}

