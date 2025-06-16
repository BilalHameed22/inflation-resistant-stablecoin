#![allow(unexpected_cfgs)]
// #![feature(trivial_bounds)]
// #[cfg(feature = "idl-build")]

use solana_sdk_ids::system_program;
use static_assertions::const_assert_eq;
use std::mem::size_of;
use bytemuck::{Zeroable, /* cast_slice, */ bytes_of_mut};
use anchor_lang::prelude::*;
// use anchor_lang::*;

use crate::pricing::Stablecoins::*;
//     use super::*;
//     use crate::pricing::{Stablecoins, BACKING_COUNT};

// The number of stablecoins that are currently supported by the IRMA program.
pub const BACKING_COUNT: usize = Stablecoins::USDE as usize; // 6
pub const BUMP_VALUE: u8 = 13; // Bump seed for PDA

// All currently existing stablecoins with about $100 M in circulation
// are supported. This list is not exhaustive and will be updated as new
// stablecoins are added to the market.
// Initially, we will support only those stablecoins that exist
// on the Solana blockchain (the first six below). 
// #[derive(Debug, AnchorSerialize, AnchorDeserialize, Zeroable, Pod, Clone, Copy, PartialEq, Eq)]
#[derive(Debug, Zeroable, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
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
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Stablecoins::USDT),
            1 => Some(Stablecoins::USDC),
            2 => Some(Stablecoins::USDS),
            3 => Some(Stablecoins::PYUSD),
            4 => Some(Stablecoins::USDG),
            5 => Some(Stablecoins::FDUSD),
            6 => Some(Stablecoins::USDE),
            7 => Some(Stablecoins::USDP),
            8 => Some(Stablecoins::SUSD),
            9 => Some(Stablecoins::ZUSD),
            10 => Some(Stablecoins::USDR),
            11 => Some(Stablecoins::DAI),
            12 => Some(Stablecoins::USD1),
            _ => None,
        }
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


/// IRMA pricing module
/// FIXME: the decimals are all assumed to be zero, which is not true for all stablecoins.

    pub fn initialize(ctx: &Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        let state = &mut ctx.accounts.state.load_init()?;

        state.mint_price = [1.0; EnumCount as usize]; // Initialize with 1.0 for each stablecoin
        msg!("Vec length: {:?}", state.mint_price.len());
        state.irma_in_circulation = [1; EnumCount as usize];
        state.backing_reserves = [0; EnumCount as usize];
        // USDR and USD1 are not yet in Solana, so we set their decimals to 
        // the following are also set to 0 (disabled): USDE, USDP, SUSD, ZUSD, and DAI.
        state.backing_decimals = [6, 6, 6, 6, 6, 6, 0, 0, 0, 0, 0, 0, 0];
        Ok(())
    }

    pub fn hello(ctx: &Context<IrmaCommon>) -> Result<()> {
        let mut state = ctx.accounts.state.load_mut()?;
        if state.mint_price.len() == 0 {
            state.mint_price = [1.0; EnumCount as usize];
            state.backing_reserves = [0; EnumCount as usize];
            state.irma_in_circulation = [0; EnumCount as usize];
        }
        msg!("State initialized with mint prices: {:?}", state.mint_price);
        msg!("Backing reserves: {:?}", state.backing_reserves);
        msg!("Irma in circulation: {:?}", state.irma_in_circulation);
        msg!("Program ID: {:?}", ctx.program_id);
        msg!("Hello world...");
        Ok(())
    }

    /// IrmaCommon of IRMA expressed in terms of a given quote token.
    /// This should be called for every backing stablecoin supported, only once per day
    /// because Truflation updates the inflation data only once per day.
    pub fn set_mint_price(ctx: &Context<IrmaCommon>, quote_token: Stablecoins, mint_price: f64) -> Result<()> {
        let mut state = ctx.accounts.state.load_mut()?;
        require!(state.backing_decimals[quote_token as usize] > 0, CustomError::InvalidQuoteToken);
        
        let curr_price = state.mint_price.get_mut(quote_token as usize).unwrap();
        require!(mint_price > 0.0, CustomError::InvalidAmount);
        *curr_price = mint_price;
        Ok(())
    }

    /// Mint IRMA tokens for a given amount of quote token.
    /// FIXME: Currently assumes that decimal point is zero digits for both IRMA and quote token.
    pub fn mint_irma(ctx: &Context<IrmaCommon>, quote_token: Stablecoins, amount: u64) -> Result<()> {
        require!(amount > 0, CustomError::InvalidAmount);

        let mut state = ctx.accounts.state.load_mut()?;
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
    pub fn redeem_irma(ctx: &Context<IrmaCommon>, quote_token: Stablecoins, irma_amount: u64) -> Result<()> {
        let mut state = ctx.accounts.state.load_mut()?;
        require!(state.backing_decimals[quote_token as usize] > 0, CustomError::InvalidQuoteToken);

        // msg!("Redeeming {} IRMA for {}, data: {:?}", irma_amount, quote_token.to_string(), ctx.accounts.state.deref().data[..8]);

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
        #[account(init, space = State::LEN, payer = irma_admin, seeds = [b"state"], bump)]
        pub state: AccountLoader<'info, State>,
        #[account(mut)]
        pub irma_admin: Signer<'info>,
        #[account(address = system_program::ID)]
        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct IrmaCommon<'info> {
        #[account(init, space = State::LEN, payer = trader, seeds = [b"state"], bump)]
        pub state: AccountLoader<'info, State>,
        #[account(mut)]
        pub trader: Signer<'info>,
        #[account(address = system_program::ID)]
        pub system_program: Program<'info, System>,
    }

    #[account(zero_copy)]
    #[derive(Debug)]
    pub struct State {
        pub mint_price: [f64; EnumCount as usize], // mint price of IRMA in terms of the backing stablecoin
        pub backing_reserves: [u64; EnumCount as usize], // backing reserves in terms of the backing stablecoin
        pub irma_in_circulation: [u64; EnumCount as usize],
        pub backing_decimals: [u64; EnumCount as usize], // need only u8, but for alignment reasons we use u64
        pub bump: u8, // Bump seed for PDA
        pub padding: [u8; 7], // padding to make the size of the struct 25 * EnumCount + 8
    }

    // Alternative implementation that allows for easy addition of new stablecoins
    //
    // #[account(zero_copy)]
    // #[derive(Debug)]
    // pub struct StableState {
    //     pub symbol: [char; 4], // symbol of the stablecoin, e.g. "USDT"
    //     pub mint_address: Pubkey, // mint address of the stablecoin
    //     pub backing_decimals: u64, // need only u8, but for alignment reasons we use u64
    //     pub mint_price: f64, // mint price of IRMA in terms of the backing stablecoin
    //     pub backing_reserves: u64, // backing reserves in terms of the backing stablecoin
    //     pub irma_in_circulation: u64,
    // }

    // #[account(zero_copy)]
    // #[derive(Debug)]
    // pub struct StateVectors {
    //     pub vectors: Vec<StableState>,
    //     pub bump: u8, // Bump seed for PDA
    //     pub padding: [u8; 7], // padding to make the size of the struct 25 * EnumCount + 8
    // }

    // StatesEnum::USDT::StableState! {
    // pub const USDT: StableState = StableState {
    //     symbol: ['U', 'S', 'D', 'T'],
    //     mint_address: pubkey!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6"), // FIXME: wrong pubkey
    //     backing_decimals: 6,
    //     mint_price: 1.0,
    //     backing_reserves: 0,
    //     irma_in_circulation: 1,
    // };

    // pub const USDC: StableState = StableState {
    //     symbol: ['U', 'S', 'D', 'C'],
    //     mint_address: pubkey!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6"), // USDC mint address on Solana
    //     backing_decimals: 6,
    //     mint_price: 1.0,
    //     backing_reserves: 0,
    //     irma_in_circulation: 1,
    // };

    // impl StableState {
    //     pub fn symbol(&self) -> &[char; 4] {
    //         self.symbol;
    //     }

    //     pub fn mint_price(&self) -> f64 {
    //         self.mint_price
    //     }

    //     // Example of a method that mutates the inner struct
    //     pub fn increment_reserve(&mut self, amount: u64) {
    //         self.backing_reserves += amount;
    //     }
    // }

    // #[account(discriminator = State::DISCRIMINATOR)]
    #[account(zero_copy)]
    #[derive(Debug)]
    pub struct StateWithDiscriminator {
        pub discriminator: [u8; 8], // discriminator for the state account
        pub state: State, // the actual state data
    }

    const_assert_eq!(
        size_of::<State>(),
        State::LEN - 8
    );


    impl State {
        pub const LEN: usize = 32*(EnumCount as usize) + 8 + 8; // extra 8 bytes for hidden discriminator
        pub fn new_mut() -> (&'static mut Self, &'static mut [u8]) {
            msg!("Creating a new State instance with hidden discriminator, size: {}", size_of::<StateWithDiscriminator>());
            let mut alloc = StateWithDiscriminator {
                discriminator: [0u8; 8],
                state: State {
                    mint_price: [1.0; EnumCount as usize],
                    backing_reserves: [0; EnumCount as usize],
                    irma_in_circulation: [1; EnumCount as usize],
                    backing_decimals: [6, 6, 6, 6, 6, 6, 0, 0, 0, 0, 0, 0, 0], // USDR and USD1 are not yet in Solana
                    bump: BUMP_VALUE,
                    padding: [0; 7],
                },
            };
            alloc.discriminator.copy_from_slice(&<State as Discriminator>::DISCRIMINATOR);
            let disc: &'static mut [u8] = bytes_of_mut(Box::leak(Box::new(alloc)));
            (&mut Box::leak(Box::new(alloc)).state, disc)
        }
        // pub fn from(buffer: &mut [u8; State::LEN]) -> Self {
        //     // Deserialize the buffer into a State instance
        //     if buffer.len() >= State::LEN {
        //         self.discriminator.copy_from_slice(&buffer[8..16]);

        //         let mint_price_bytes = &buffer[16..16 + 8 * EnumCount as usize];
        //         let mint_price_f64: &[f64] = cast_slice(mint_price_bytes);
        //         self.mint_price.copy_from_slice(mint_price_f64);

        //         let backing_reserves_bytes = &buffer[16 + 8 * EnumCount as usize..16 + 16 * EnumCount as usize];
        //         let backing_reserves_u64: &[u64] = cast_slice(backing_reserves_bytes);
        //         self.backing_reserves.copy_from_slice(backing_reserves_u64);

        //         let irma_in_circulation_bytes = &buffer[16 + 16 * EnumCount as usize..16 + 24 * EnumCount as usize];
        //         let irma_in_circulation_u64: &[u64] = cast_slice(irma_in_circulation_bytes);
        //         self.irma_in_circulation.copy_from_slice(irma_in_circulation_u64);

        //         let backing_decimals_bytes = &buffer[16 + 24 * EnumCount as usize..16 + 32 * EnumCount as usize];
        //         let backing_decimals_u64: &[u64] = cast_slice(backing_decimals_bytes);
        //         self.backing_decimals.copy_from_slice(backing_decimals_u64);
        //         self.bump = buffer[16 + 32 * EnumCount as usize];
        //     }
        //     state
        // }

        // pub fn try_from(buffer: &[u8]) -> Result<Self> {
        //     if buffer.len() < State::LEN {
        //         return Err(ErrorCode::InvalidState.into());
        //     }
        //     let mut state = State::new();
        //     state.discriminator.copy_from_slice(&buffer[0..8]);
        //     if state.discriminator != <State as Discriminator>::DISCRIMINATOR {
        //         return Err(ErrorCode::InvalidState.into());
        //     }
        //     state = State::from(cast_slice(buffer));
        //     Ok(state)
        // }
    }
    // #[account(zero_copy)]
    // pub struct IrmaCommonBumps { bump: u8 };


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
            let price_differences : Vec<f64> = self.backing_reserves
                .iter()
                .enumerate()
                .filter_map(|(i, reserve)| {
                    let circulation = self.irma_in_circulation[i];
                    let redemption_price = *reserve as f64 / circulation as f64;
                    let mint_price = self.mint_price[i];
                    if mint_price == 0.0 || self.backing_decimals[i] == 0 {
                        msg!("Skipping {}: mint_price is 0.0 or backing_decimals is 0", Stablecoins::from_index(i).unwrap().to_string());
                        return Some(0.0);
                    }
                    count += 1;
                    let x: f64 = mint_price - redemption_price;
                    average_diff += x;
                    Some(x)
                })
                .collect::<Vec<f64>>();
            msg!("Price differences: {:?}", price_differences);
            if count == 0 {
                msg!("No price differences found, returning early.");
                return Ok(());
            }
            average_diff /= count as f64;
            msg!("Average price difference: {}", average_diff);

            let min_diff: f64 = 0.1; // price differences below this are ignored

            let mut max_price_diff: f64 = average_diff;
            let mut first_target: Stablecoins = quote_token;
            for (i, price_diff) in price_differences.iter().enumerate() {
                msg!("{}: {}", i, *price_diff);
                if (*price_diff - max_price_diff).abs() > min_diff {
                    max_price_diff = *price_diff;
                    first_target = Stablecoins::from_index(i).unwrap();
                }
            }
            // max_price_diff = (max_price_diff - average_diff).abs();
            // msg!("Max token: {}", first_target.to_string());
            msg!("Max price diff: {}", max_price_diff);

            let circulation: &mut u64 = self.irma_in_circulation.get_mut(quote_token as usize).unwrap();
            let reserve: &mut u64 = self.backing_reserves.get_mut(quote_token as usize).unwrap();
            let mut redemption_price: f64 = *reserve as f64 / *circulation as f64;

            // if max price diff does not deviate much from average diff or all inflation-adjusted prices 
            // are less than the redemption prices, then reductions pertain to quote_token only.
            if ((max_price_diff - average_diff).abs() < min_diff) || (average_diff < 0.0) {
                if price_differences[quote_token as usize] > 0.0 || first_target == quote_token {
                    msg!("No significant price difference, adjusting only the quote token.");
                    // If the price difference is positive, it means that the mint price is higher than the redemption price;
                    // in this case, we need to reduce IRMA in circulation by the irma_amount.
                    require!(*circulation >= irma_amount, CustomError::InsufficientCirculation);
                    *circulation -= irma_amount;
                    let backing_amount: u64 = (irma_amount as f64 * redemption_price) as u64;
                    require!(*reserve >= backing_amount, CustomError::InsufficientReserve);
                    *reserve -= backing_amount;
                    msg!("Redeemed {} IRMA for {} backing tokens.", irma_amount, backing_amount);
                } else {
                    // If the price difference is negative, it means that the mint price is lower than the redemption price;
                    // in this case, we need to set the redemption price eq to the mint price in order to preserve the backing.
                    // We also do not reduce IRMA in circulation, which effectively means that we are still draining the reserve,
                    // but not by much, while the reduction in the ratio of reserve to IRMA in circulation (normally the
                    // redemption price) goes down faster than if we also reduced IRMA in circulation. 
                    require!(irma_amount <= *circulation, CustomError::InsufficientCirculation);
                    require!(redemption_price > self.mint_price[quote_token as usize], CustomError::InvalidBacking);
                    redemption_price = self.mint_price[quote_token as usize];
                    let backing_amount: u64 = (irma_amount as f64 * redemption_price) as u64;
                    require!(*reserve >= backing_amount, CustomError::InsufficientReserve);
                    *reserve -= backing_amount;
                    msg!("Redeemed {} IRMA for {} backing tokens.", irma_amount, backing_amount);
                }
                msg!("New reserve for {}: {}", quote_token.to_string(), *reserve);
                msg!("New circulation for {}: {}", quote_token.to_string(), *circulation);
                return Ok(());
            }
            // All the following code is for the normal case, in which the mint price is higher than or equal to the
            // redemption price.
            msg!("First target for normal adjustments: {}", first_target.to_string());

            // no matter what, we need to reduce the second_target reserve
            let backing_amount: u64 = (irma_amount as f64 * redemption_price) as u64;
            require!(*reserve >= backing_amount, CustomError::InsufficientReserve);
            *reserve -= backing_amount;

            let first_circulation: u64 = self.irma_in_circulation[first_target as usize];
            let second_circulation: u64 = self.irma_in_circulation[quote_token as usize];

            // if we don't have enough reserve to redeem the irma_amount, just error out;
            // we can't allow redemption from a reserve that is smaller than the irma_amount.
            require!(irma_amount < second_circulation, CustomError::InsufficientCirculation);
            require!(irma_amount < first_circulation, CustomError::InsufficientCirculation);

            let first_price: f64 = self.mint_price[first_target as usize];
            let second_price: f64 = self.mint_price[quote_token as usize];
            let first_reserve: u64 = self.backing_reserves[first_target as usize];
            let second_reserve: u64 = self.backing_reserves[quote_token as usize];

            let first_price_diff: f64 = first_price - (first_reserve / first_circulation) as f64;
            let post_second_price_diff: f64 = second_price - (second_reserve as f64 - irma_amount as f64 / second_price) / second_circulation as f64;
            let post_first_price_diff: f64 = first_price - (first_reserve as f64 / (first_circulation - irma_amount) as f64);

            if first_price_diff <= post_first_price_diff {
                msg!("--> First price diff is less than or equal to post first price diff, adjusting second circulation only.");
                // if irma_amount is such that conditions would remain the same post adjustment
                // we can just subtract from the second_circulation
                let second_circulation = self.irma_in_circulation.get_mut(quote_token as usize).unwrap();
                *second_circulation -= irma_amount;
            } else
            if post_first_price_diff <= post_second_price_diff {
                msg!("--> Post first price diff is less than or equal to second price diff, 
                        adjusting first circulation only.");
                // if irma_amount is such that conditions would remain the same post adjustment
                // we can just subtract from the first_circulation
                let first_circulation = self.irma_in_circulation.get_mut(first_target as usize).unwrap();
                *first_circulation -= irma_amount;
            } else {
                msg!("--> First and second prices are close enough, adjusting both circulations linearly.");
                // Do simple linear adjustment of both first and second circulations
                let adjustment_amount: f64 = irma_amount as f64 * (first_price_diff - post_second_price_diff) / (first_price_diff + post_second_price_diff);
                msg!("Adjustment amount: {}", adjustment_amount);
                require!(adjustment_amount > 0.0, CustomError::InvalidAmount);
                require!(adjustment_amount <= irma_amount as f64, CustomError::InvalidAmount);
                msg!("Adjusting first circulation by {} and second circulation by {}", adjustment_amount.ceil(), irma_amount as f64 - adjustment_amount.ceil());
                let first_circulation: &mut u64 = self.irma_in_circulation.get_mut(first_target as usize).unwrap();
                *first_circulation -= adjustment_amount.ceil() as u64;
                let second_circulation: &mut u64 = self.irma_in_circulation.get_mut(quote_token as usize).unwrap();
                *second_circulation -= irma_amount - adjustment_amount.ceil() as u64;
            } 
            msg!("New reserve for {}: {}", first_target.to_string(), self.backing_reserves[first_target as usize]);
            msg!("New reserve for {}: {}", quote_token.to_string(), self.backing_reserves[quote_token as usize]);
            msg!("New circulation for {}: {}", first_target.to_string(), self.irma_in_circulation[first_target as usize]);
            msg!("New circulation for {}: {}", quote_token.to_string(), self.irma_in_circulation[quote_token as usize]);
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

