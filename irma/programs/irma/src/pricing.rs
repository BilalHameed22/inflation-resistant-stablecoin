#![allow(unexpected_cfgs)]
// #![feature(trivial_bounds)]
// #[cfg(feature = "idl-build")]
use std::string::String;
// se std::vec::Vec;
use std::option::Option;
// use anchor_lang_idl_spec::IdlType::Option as IdlOption;
// use anchor_lang_idl_spec::IdlType::Pubkey as IdlPubkey;
use anchor_lang_idl_spec::{
    IdlType,
    IdlTypeDef, 
    IdlTypeDefTy, 
    IdlField, 
    IdlGenericArg, 
    IdlDefinedFields, 
    IdlSerialization,
};
use anchor_lang::*;
// use anchor_lang::system_program::ID;
use anchor_lang::prelude::*;
use std::collections::BTreeMap;

use crate::Initialize;
use crate::IrmaCommon;
// use crate::StateMap;
// use crate::StableState;
// use crate::IRMA;

// The number of stablecoins that are initially supported by the IRMA program.
pub const BACKING_COUNT: usize = 6 as usize;
// Maximum number of stablecoins supported
pub const MAX_BACKING_COUNT: usize = 100;

declare_id!("8zs1JbqxqLcCXzBrkMCXyY2wgSW8uk8nxYuMFEfUMQa6");

/// IRMA module
/// FIXME: the decimals are all assumed to be zero, which is not true for all stablecoins.


pub fn initialize_pricing(ctx: Context<Initialize>) -> Result<()> {
    msg!("Greetings from: {:?}", ctx.program_id);
    let state = &ctx.accounts.state;
    if state.reserves.len() > 0 {
        msg!("State already initialized, skipping init...");
        return Ok(());
    }
    *ctx.accounts.state = StateMap::new();
    let state = &mut ctx.accounts.state;
    state.bump = 13u8; // InitializeBumps::bump(&ctx.bumps).unwrap_or(0);
    msg!("State initialized with bump: {}", state.bump);

    state.add_initial_stablecoins()?;
    msg!("Initial stablecoins added to the state.");

    Ok(())
}

/// The whole purpose for using a BTreeMap is to allow for easy addition of new stablecoins.
pub fn add_stablecoin(
        ctx: Context<Initialize>, 
        symbol: &str, 
        mint_address: prelude::Pubkey,
        backing_decimals: u8) -> Result<()> 
{
    let state = &mut ctx.accounts.state;
    if state.reserves.len() >= MAX_BACKING_COUNT {
        msg!("Maximum number of stablecoins reached.");
        return Err(error!(CustomError::InvalidBacking));
    }
    let stablecoin = StableState::new(symbol, mint_address, backing_decimals as u64).unwrap();
    state.add_stablecoin(stablecoin.clone());
    msg!("Added stablecoin: {:?}", stablecoin);
    Ok(())
}

/// Remove a stablecoin from the reserves by its symbol.
pub fn remove_stablecoin(ctx: Context<Initialize>, symbol: &str) -> Result<()> {
    let state = &mut ctx.accounts.state;
    if !state.contains_stablecoin(symbol) {
        msg!("Stablecoin {} not found in reserves.", symbol);
        return Err(error!(CustomError::InvalidBacking));
    }
    state.remove_stablecoin(symbol);
    msg!("Removed stablecoin: {}", symbol);
    Ok(())
}

/// Deactivate a reserve stablecoin.
pub fn deactivate_stablecoin(ctx: Context<Initialize>, symbol: &str) -> Result<()> {
    let state = &mut ctx.accounts.state;
    if !state.contains_stablecoin(symbol) {
        msg!("Stablecoin {} not found in reserves.", symbol);
        return Err(error!(CustomError::InvalidBacking));
    }
    state.deactivate_stablecoin(symbol);
    msg!("Deactivated stablecoin: {}", symbol);
    Ok(())
}

pub fn validate_reserve(ctx: Context<IrmaCommon>, reserve: &str) -> Result<()> {
    let state = &mut ctx.accounts.state;
    if state.reserves.len() == 0 {
        msg!("State not initialized, call initialize first...");
        return Err(error!(CustomError::InvalidBacking));
    }
    let usdt = state.get_stablecoin(reserve).ok_or(CustomError::InvalidQuoteToken)?;
    msg!("USDT initialized with mint prices: {:?}", usdt.mint_price);
    msg!("Total USDT reserves: {:?}", usdt.backing_reserves);
    msg!("Irma in circulation for USDT: {:?}", usdt.irma_in_circulation);
    msg!("Program ID: {:?}", ctx.program_id);
    msg!("Hello world...");
    Ok(())
}

fn validate_params(reserves: BTreeMap<String, StableState>, quote_token: &str) -> Result<()> {
    require!(reserves.len() > 0, CustomError::InvalidReserveList);
    require!(reserves.contains_key(quote_token), CustomError::InvalidQuoteToken);
    let stablecoin = reserves.get(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
    require!(stablecoin.active, CustomError::InvalidQuoteToken);
    require!(stablecoin.backing_decimals > 0, CustomError::InvalidQuoteToken);
    require!(stablecoin.mint_price > 0.0, CustomError::InvalidAmount);
    require!(stablecoin.irma_in_circulation > 0u64, CustomError::InsufficientCirculation);
    Ok(())
}

/// IrmaCommon of IRMA expressed in terms of a given quote token.
/// This should be called for every backing stablecoin supported, only once per day
/// because Truflation updates the inflation data only once per day.
pub fn set_mint_price(ctx: Context<IrmaCommon>, quote_token: &str, mint_price: f64) -> Result<()> {
    let reserves = &mut ctx.accounts.state.reserves;
    validate_params(reserves.clone(), quote_token)?;
    require!(mint_price > 0.0, CustomError::InvalidAmount);

    let stablecoin = reserves.get_mut(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
    stablecoin.mint_price = mint_price;
    Ok(())
}

/// Mint IRMA tokens for a given amount of quote token.
/// Input amount is  in quote token's smallest unit (e.g. 1 USDT = 10^6, 1 USDC = 10^6, etc.)
/// The mint price is the price of IRMA in terms of the quote token, which is set by the Truflation oracle.
pub fn mint_irma(ctx: Context<IrmaCommon>, quote_token: &str, amount: u64) -> Result<()> {
    let reserves = &mut ctx.accounts.state.reserves;
    validate_params(reserves.clone(), quote_token)?;

    if amount == 0 { return Ok(()); };

    let curr_price: f64 = reserves[quote_token].mint_price;
    let amount = (amount as f64 / (10.0_f64).powf(reserves[quote_token].backing_decimals as f64)) as f64;

    let stablecoin = reserves.get_mut(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
    stablecoin.backing_reserves += amount.ceil() as u64; // backing should not have a fractional part
    stablecoin.irma_in_circulation += (amount / curr_price).ceil() as u64;

    Ok(())
}

/// RedeemIRMA - user surrenders IRMA in irma_amount, expecting to get back quote_token according to redemption price.
/// FIXME: If resulting redemption price increases by more than 0.0000001, then actual redemption price 
/// should be updated immediately.
pub fn redeem_irma(ctx: Context<IrmaCommon>, quote_token: &str, irma_amount: u64) -> Result<()> {
    let reserves = &mut ctx.accounts.state.reserves;
    validate_params(reserves.clone(), quote_token)?;

    if irma_amount == 0 { return Ok(()) };

    let state = reserves[quote_token].clone();
    // There is a redemption rule: every redemption is limited to 100k IRMA or 10% of the IRMA in circulation (for
    // the quote token) whichever is smaller.
    let circulation: u64 = state.irma_in_circulation;
    // let circulation: u64 = state.irma_in_circulation * (10u64.pow(state.backing_decimals as u32));
    // require!(circulation > 0, CustomError::InsufficientCirculation);
    let irma_amount = (irma_amount as f64 / (10.0_f64).powf(IRMA.backing_decimals as f64)) as f64;
    require!((irma_amount <= 100_000.0) && (irma_amount <= circulation as f64 / 10.0), CustomError::InvalidIrmaAmount);

    ctx.accounts.state.reduce_circulations(quote_token, irma_amount.ceil() as u64)?;

    Ok(())
}

// #[account]
// #[derive(InitSpace)]
// #[derive(Debug)]
// pub struct State {
//     #[max_len(BACKING_COUNT)]
//     pub mint_price: Vec<f64>,
//     #[max_len(BACKING_COUNT)]
//     pub backing_reserves: Vec<u64>,
//     #[max_len(BACKING_COUNT)]
//     pub backing_decimals: Vec<u8>,
//     #[max_len(BACKING_COUNT)]
//     pub irma_in_circulation: Vec<u64>,
//     pub bump: u8,
// }

/// Alternative implementation that allows for easy addition of new stablecoins
/// Each stablecoin struct uses 80 bytes, with estimated 40 bytes for BTreeMap overhead.
#[account]
#[derive(PartialEq, Debug)]
pub struct StableState {
    pub symbol: String, // symbol of the stablecoin, e.g. "USDT"
    pub mint_address: Pubkey, // mint address of the stablecoin
    pub backing_decimals: u64, // need only u8, but for alignment reasons we use u64
    pub mint_price: f64, // mint price of IRMA in terms of the backing stablecoin
    pub backing_reserves: u64, // backing reserves is in whole numbers (no decimals)
    pub irma_in_circulation: u64, // in whole numbers (no decimals)
    pub active: bool, // whether the stablecoin is active or not
    pub extra: [u8; 7], // padding to make the size of the struct 25 * EnumCount + 8
}

#[account]
#[derive(PartialEq, Debug)]
pub struct StateMap {
    pub reserves: BTreeMap<String, StableState>,
    pub bump: u8, // Bump seed for PDA
    pub padding: [u8; 7], // padding to make the size of the struct 25 * EnumCount + 8
}

/// Immutable data for IRMA itself.
pub const IRMA: StableState = StableState {
    symbol: String::new(), // symbol of the stablecoin, e.g. "IRMA"
    mint_address: pubkey!("irmacFBRx7148dQ6qq1zpzUPq57Jr8V4vi5eXDxsDe1"), // IRMA mint address on Solana
    backing_decimals: 6,
    mint_price: 1.0,
    backing_reserves: 0u64,
    irma_in_circulation: 1u64,
    active: false, // IRMA cannot be a reserve backing of itself
    extra: [0; 7], // padding
};

pub trait MapTrait {
    fn size(&self) -> usize;
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()>;
    // fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> where Self: Sized;
    fn get_full_path() -> String;
    fn create_type() -> Option<IdlTypeDef>;
    fn insert_types(_types: &mut BTreeMap<String, IdlTypeDef>);
}

impl MapTrait for Pubkey {
    fn size(&self) -> usize {
        // Calculate the size of the Pubkey
        std::mem::size_of::<Self>()
    }
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Serialize the Pubkey to the writer
        writer.write_all(self.as_ref())
    }
    fn get_full_path() -> String {
        // Return the full path of the Pubkey
        "anchor_lang::prelude::Pubkey".to_string()
    }
    fn create_type() -> Option<IdlTypeDef> {
        // Create an IdlTypeDef for the Pubkey
        Some(IdlTypeDef {
            name: "Pubkey".to_string(),
            ty: IdlTypeDefTy::Type { alias: IdlType::Pubkey },
            docs: vec![],
            repr: None,
            generics: vec![],
            serialization: IdlSerialization::Borsh,
        })
    }
    fn insert_types(_types: &mut BTreeMap<String, IdlTypeDef>) {
        // Insert the Pubkey type into the types map
        _types.insert("Pubkey".to_string(), Self::create_type().unwrap());
    }
}

impl MapTrait for BTreeMap<String, StableState> {
    fn size(&self) -> usize {
        // Calculate the size of the BTreeMap
        self.iter().map(|(k, _v)| k.len() + std::mem::size_of::<StableState>()).sum()
    }
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Serialize the BTreeMap to the writer
        for (key, value) in self.iter() {
            writer.write_all(key.as_bytes())?;
            value.serialize(writer)?;
        }
        Ok(())
    }
    // fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
    //     // Deserialize the BTreeMap from the reader
    //     let mut map = BTreeMap::new();
    //     let mut key = String::new();
    //     while let Ok(_) = reader.read_to_string(&mut key) {
    //         if key.is_empty() { break; }
    //         let mut map_value_bytes = [0u8; std::mem::size_of::<StableState>()] as String;
    //         let result = reader.read_to_string(&mut map_value_bytes);
    //         match result {
    //             Ok(_) => {},
    //             Err(e) => {
    //                 if e.kind() == std::io::ErrorKind::UnexpectedEof {
    //                     break; // End of file reached
    //                 } else {
    //                     return Err(e); // Other error
    //                 }
    //             }
    //         }
    //         let value = StableState::deserialize(&map_value_bytes)?; // reader - expected `&mut &[u8]`, found `&mut R
    //         map.insert(key.clone(), value);
    //         key.clear();
    //     }
    //     Ok(map)
    // }
    fn get_full_path() -> String {
        // Return the full path of the BTreeMap
        "std::collections::BTreeMap".to_string()
    }
    // FIXME: Why is StableState duplicated here? (see insert_types)
    fn create_type() -> Option<IdlTypeDef> {
        // Create an IdlTypeDef for the BTreeMap
        Some(IdlTypeDef {
            name: "BTreeMap".to_string(),
            ty: IdlTypeDefTy::Type {
                alias: IdlType::Defined {
                    name: "BTreeMap".to_string(),
                    generics: vec![
                        IdlGenericArg::Type { ty: IdlType::String }, // Key type is String
                        IdlGenericArg::Type { 
                            ty: IdlType::Defined {
                                name: "StableState".to_string(),
                                generics: vec![
                                    IdlGenericArg::Type { ty: IdlType::String },
                                    IdlGenericArg::Type { ty: IdlType::Pubkey },
                                    IdlGenericArg::Type { ty: IdlType::U64 },
                                    IdlGenericArg::Type { ty: IdlType::F64 },
                                    IdlGenericArg::Type { ty: IdlType::U64 },
                                    IdlGenericArg::Type { ty: IdlType::U64 },
                                    IdlGenericArg::Type { ty: IdlType::Bool },
                                ],
                            },
                        }
                    ],
                }
            },
            docs: vec![],
            repr: None,
            generics: vec![],
            serialization: IdlSerialization::Borsh,
        })
    }
    fn insert_types(_types: &mut BTreeMap<String, IdlTypeDef>) {
        // Insert the BTreeMap type into the types map
        _types.insert("BTreeMap".to_string(), Self::create_type().unwrap());
        _types.insert("StableState".to_string(), IdlTypeDef {
            name: "StableState".to_string(),
            ty: IdlTypeDefTy::Struct {
                // fields: IdlOption::<IdlDefinedFields::Named> (vec![
                fields: Some(IdlDefinedFields::Named(vec![
                    IdlField{ name: "symbol".to_string(), ty: IdlType::String, docs: vec![] },
                    IdlField { name: "mint_address".to_string(), ty: IdlType::Pubkey, docs: vec![] },
                    IdlField { name: "backing_decimals".to_string(), ty: IdlType::U64, docs: vec![] },
                    IdlField { name: "mint_price".to_string(), ty: IdlType::F64, docs: vec![] },
                    IdlField { name: "backing_reserves".to_string(), ty: IdlType::U64, docs: vec![] },
                    IdlField { name: "irma_in_circulation".to_string(), ty: IdlType::U64, docs: vec![] },
                    IdlField { name: "active".to_string(), ty: IdlType::Bool, docs: vec![] },
                ])),
            },
            docs: vec![],
            repr: None,
            generics: vec![],
            serialization: IdlSerialization::Borsh,
        });
    }
}

impl StableState {

    pub fn new(symbol: &str, mint_address: prelude::Pubkey, backing_decimals: u64) -> Result<Self> {
        // const len: usize = symbol.to_bytes().len();
        require!(symbol.len() <= 8, CustomError::InvalidBackingSymbol);
        require!(mint_address != prelude::Pubkey::default(), CustomError::InvalidBackingAddress);
        require!(backing_decimals > 0, CustomError::InvalidBacking);
        Ok(StableState {
            symbol: symbol.to_string(), // symbol of the stablecoin, e.g. "USDT"
            mint_address,
            backing_decimals,
            mint_price: 1.0, // default mint price is 1.0
            backing_reserves: 0,
            irma_in_circulation: 1,
            active: true,
            extra: [0; 7], // for future use
        })
    }
}

impl StateMap {
    pub fn new() -> Self {
        StateMap {
            reserves: BTreeMap::new(),
            bump: 0,
            padding: [0; 7], // padding to make the size of the struct 25 * EnumCount + 8
        }
    }

    pub fn add_stablecoin(&mut self, stablecoin: StableState) {
        if self.contains_stablecoin(&stablecoin.symbol) {
            msg!("MapTrait {} already exists in reserves, skipping addition.", stablecoin.symbol);
            return;
        }
        let symbol = stablecoin.clone().symbol; // Get the symbol from the stablecoin
        self.reserves.insert(symbol, stablecoin);
    }

    pub fn get_stablecoin(&self, symbol: &str) -> Option<StableState> {
        if !self.contains_stablecoin(symbol) {
            msg!("MapTrait {} not found in reserves.", symbol);
            return None;
        }
        self.reserves.get(symbol).map(|s| {
            // Ensure the stablecoin is immutable
            let stablecoin = s;
            // Return a reference to the stablecoin
            stablecoin
        }).cloned()
    }

    pub fn get_mut_stablecoin(&mut self, symbol: &str) -> Option<&mut StableState> {
        if !self.contains_stablecoin(symbol) {
            msg!("MapTrait {} not found in reserves.", symbol);
            return None;
        }
        self.reserves.get_mut(symbol).map(|s| {
            // Ensure the stablecoin is mutable
            let stablecoin = &mut *s;
            // Return a mutable reference to the stablecoin
            stablecoin
        })
    }

    pub fn get_stablecoin_symbol(&self, mint_address: prelude::Pubkey) -> Option<String> {
        for (symbol, stablecoin) in &self.reserves {
            if stablecoin.mint_address == mint_address {
                return Some(symbol.clone());
            }
        }
        None
    }

    pub fn remove_stablecoin(&mut self, symbol: &str) -> Option<StableState> {
        self.reserves.remove(symbol) // .into_iter().copied()
    }

    pub fn deactivate_stablecoin(&mut self, symbol: &str) {
        if let Some(stablecoin) = self.reserves.get_mut(symbol) {
            stablecoin.active = false;
            msg!("Deactivated stablecoin: {}", symbol);
        } else {
            msg!("Stablecoin {} not found in reserves.", symbol);
        }
    }

    pub fn contains_stablecoin(&self, symbol: &str) -> bool {
        self.reserves.contains_key(symbol)
    }
    
    pub fn len(&self) -> usize {
        self.reserves.len()
    }

    pub fn add_initial_stablecoins(&mut self) -> Result<()> {
        // This function is used to add initial stablecoins to the reserves.
        // It is called during the initialization of the IRMA program.
        // let usdt = StableState::new("USDT", pubkey!("Es9vMFrzaTmVRL3P15S3BtQDvVwWZEzPDk1e45sA2v6p"), 6)?;
        // self.add_stablecoin(usdt);
        
        // let usdc = StableState::new("USDC", pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), 6)?;
        // self.add_stablecoin(usdc);
        
        // let pyusd = StableState::new("PYUSD", pubkey!("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo"), 6)?;
        // self.add_stablecoin(pyusd);
        
        // let usds = StableState::new("USDS", pubkey!("USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA"), 6)?;
        // self.add_stablecoin(usds);
        
        // let usdg = StableState::new("USDG", pubkey!("2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH"), 6)?;
        // self.add_stablecoin(usdg);
        
        // let fdusd = StableState::new("FDUSD", pubkey!("9zNQRsGLjNKwCUU5Gq5LR8beUCPzQMVMqKAi3SSZh54u"), 6)?;
        // self.add_stablecoin(fdusd);

        // Ok(())
        //     symbol: Box::new(symbols[0].to_string()), // symbol of the stablecoin, e.g. "USDT"
        //     mint_address: pubkey!("Es9vMFrzaTmVRL3P15S3BtQDvVwWZEzPDk1e45sA2v6p"), // USDT mint address on Solana
        let usdt = StableState::new(
            "USDT",
            pubkey!("Es9vMFrzaTmVRL3P15S3BtQDvVwWZEzPDk1e45sA2v6p"), // USDT mint address on Solana
            6,
        )?;
        self.add_stablecoin(usdt);

        //     symbol: Box::new(symbols[1].to_string()), // symbol of the stablecoin, e.g. "USDC"
        //     mint_address: pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC mint address on Solana
        let usdc = StableState::new(
            "USDC",
            pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDT mint address on Solana
            6,
        )?;
        self.add_stablecoin(usdc);

        //     symbol: Box::new(symbols[2].to_string()),
        //     mint_address: pubkey!("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo"), // PYUSD mint address on Solana
        let pyusd = StableState::new(
            "PYUSD",
            pubkey!("2b1kV6DkPAnxd5ixfnxCpjxmKwqjjaYmCZfHsFu24GXo"), // USDT mint address on Solana
            6,
        )?;
        self.add_stablecoin(pyusd);

        //     symbol: Box::new(symbols[3].to_string()),
        //     mint_address: pubkey!("USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA"), // USDS mint address on Solana
        let usds = StableState::new(
            "USDS",
            pubkey!("USDSwr9ApdHk5bvJKMjzff41FfuX8bSxdKcR81vTwcA"), // USDT mint address on Solana
            6,
        )?;
        self.add_stablecoin(usds);

        //     symbol: Box::new(symbols[4].to_string()),
        //     mint_address: pubkey!("2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH"), // USDG mint address on Solana
        let usdg = StableState::new(
            "USDG",
            pubkey!("2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH"), // USDT mint address on Solana
            6,
        )?;
        self.add_stablecoin(usdg);

        //     symbol: Box::new(symbols[5].to_string()),
        //     mint_address: pubkey!("9zNQRsGLjNKwCUU5Gq5LR8beUCPzQMVMqKAi3SSZh54u"), // FDUSD mint address on Solana
        let fdusd = StableState::new(
            "FDUSD",
            pubkey!("9zNQRsGLjNKwCUU5Gq5LR8beUCPzQMVMqKAi3SSZh54u"), // USDT mint address on Solana
            6,
        )?;
        self.add_stablecoin(fdusd);

        msg!("BTreeMap length: {:?}", self.reserves.len());
        Ok(())
    }  

    /// ReduceCirculations implementation
    /// This now deals with mint_price being less than redemption_price (a period of deflation).
    /// If the price of the underlying reserve goes up with respect to USD, its exchange rate with IRMA
    /// would improve (i.e. IRMA would be worth less in terms of the reserve). In this case, the system
    /// would be expected to have a higher redemption price for IRMA than mint price; however, because
    /// the objective is always to preserve the backing, the system will not allow the mint price 
    /// to be less than the redemption price. Instead, it will simply set the redemption price to the mint price.
    /// NOTE: irma_amount is now scaled down by the backing_decimals of IRMA.
    fn reduce_circulations(&mut self, quote_token: &str, irma_amount: u64) -> Result<()> {

        require!(quote_token.len() > 2, CustomError::InvalidQuoteToken);
        let reserves = &mut self.reserves;
        let clone_reserves = reserves.clone();

        // determine what this redemption does:
        // does it keep the relative spreads even, or does it skew the spreads?
        let mut count: u8 = 0;
        let mut average_diff: f64 = 0.0;
        let price_differences: BTreeMap<&String, f64> = clone_reserves.iter()
            .enumerate()
            .filter_map(|(i, reserve)| {
                let key = reserve.0;
                let reserve = reserve.1;
                let reserve = reserve.clone(); // clone to get a copy of the StableState
                // msg!("{}: {}", i, reserve.symbol.to_string());
                let circulation = reserve.irma_in_circulation;
                let redemption_price = reserve.backing_reserves as f64 / circulation as f64;
                let mint_price = reserve.mint_price;
                if mint_price == 0.0 || reserve.backing_decimals == 0 || reserve.active == false {
                    // msg!("Skipping {}: mint_price is 0.0 or backing_decimals is 0", Stablecoins::from_index(i).unwrap().to_string());
                    return Some((key, 0.0));
                }
                count += 1;
                if count != i as u8 + 1 {
                    msg!("Warning: count is not equal to index + 1, count: {}, index: {}", count, i);
                }
                let x: f64 = mint_price - redemption_price;
                average_diff += x;
                Some((key, x))
            })
            .collect();
        require!(count > 0, CustomError::InvalidBacking);
        // if count == 0 {
        //     // msg!("No price differences found, returning early.");
        //     return Ok(());
        // }
        average_diff /= count as f64;
        // msg!("Average price difference: {}", average_diff);

        let min_diff: f64 = 0.001; // price differences below this are ignored

        let mut max_price_diff: f64 = average_diff;
        let mut other_target: &String = &quote_token.to_string();
        for (_i, (key, price_diff)) in price_differences.iter().enumerate() {
            // msg!("{}: {}, max {}", i, *price_diff, max_price_diff);
            if (*price_diff - max_price_diff).abs() > min_diff && *price_diff > max_price_diff {
                max_price_diff = *price_diff;
                other_target = key;
            }
        }
        // msg!("Max token: {}", other_target.to_string());
        // msg!("Max price diff: {}", max_price_diff);

        let ro_circulation: u64 = reserves[quote_token].irma_in_circulation;
        let reserve: u64 = reserves[quote_token].backing_reserves;
        let redemption_price: f64 = reserve as f64 / ro_circulation as f64;
        let subject_adjustment: u64 = (irma_amount as f64 * redemption_price).ceil() as u64; // irma_amount is in whole numbers, so we can use it directly

        // no matter what, we need to reduce the subject reserve (quote_token)
        require!(reserve >= subject_adjustment, CustomError::InsufficientReserve);
        let mut_reserve = reserves.get_mut(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
        mut_reserve.backing_reserves -= subject_adjustment;

        // if max price diff does not deviate much from average diff or all inflation-adjusted prices 
        // are less than the redemption prices, then reductions pertain to quote_token only.
        if (average_diff.abs() < min_diff) || (average_diff < 0.0) {
            // msg!("No significant price differences found");
            if price_differences[&quote_token.to_string()] >= 0.0 || *other_target == *quote_token {
                // msg!("If quote_token m price is larger than r price, then situation is normal.");
                // If the price difference is positive, it means that the mint price is higher than the redemption price;
                // in this case, we need to reduce IRMA in circulation by the irma_amount.
                // Note that this keeps price differences the same (it's minting that adjusts redemption price).
                let circulation: u64 = reserves[quote_token].irma_in_circulation;
                require!(circulation >= irma_amount, CustomError::InsufficientCirculation);
                let mut_reserve = reserves.get_mut(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
                mut_reserve.irma_in_circulation -= irma_amount;
            } else {
                msg!("m price <= r price for quote token, adjust backing reserve only for {:?}.", quote_token);
                // If the price difference is negative, it means that the mint price is lower than the redemption price;
                // in this case, we need to set the redemption price eq to the mint price in order to preserve the backing.
                // We also do not reduce IRMA in circulation, which effectively means that we are still draining the reserve,
                // but not by much, while the reduction in the ratio of reserve to IRMA in circulation (normally the
                // redemption price) goes down faster than if we also reduced IRMA in circulation. 
                // And we're done!
            }
            // msg!("New reserve for {}: {}", quote_token.to_string(), *reserve);
            // let ro_circulation: u64 = reserves[quote_token].irma_in_circulation;
            // msg!("New circulation for {}: {}", quote_token.to_string(), ro_circulation);
            return Ok(());
        }
        // All the following code is for the semi-normal case, in which the mint price 
        // is higher than or equal to the redemption price; but the price differences
        // can be large.
        // msg!("Other target for normal adjustments: {}", other_target.to_string());

        let other_circulation: u64 = reserves[other_target].irma_in_circulation;

        // if we don't have enough reserve to redeem the irma_amount, just error out;
        // we can't allow redemption from a reserve that is smaller than the irma_amount.
        // require!(irma_amount <= *circulation, CustomError::InsufficientCirculation);

        let other_price: f64 = reserves[other_target].mint_price;
        let price: f64 = reserves[quote_token].mint_price;
        let other_reserve: u64 = reserves[other_target].backing_reserves;
        let reserve: u64 = reserves[quote_token].backing_reserves;

        let other_price_diff: f64 = other_price - (other_reserve / other_circulation) as f64;
        let ro_circulation: u64 = reserves[quote_token].irma_in_circulation;
        let post_price_diff: f64 = price - (reserve as f64 - irma_amount as f64 / price) / ro_circulation as f64;
        let post_other_price_diff: f64 = other_price - (other_reserve as f64 / (other_circulation - irma_amount) as f64);

        if other_price_diff <= post_other_price_diff {
            // msg!("--> Other price diff is less than or equal to post other price diff, adjusting second circulation only.");
            // if irma_amount is such that it could not improve the redemption price when applied to other stabecoin reserve,
            // we can just subtract from the circulation (same as normal case).
            // Note that the normal case does not change redemtion prices.
            let circulation: u64 = reserves[quote_token].irma_in_circulation;
            require!(irma_amount <= circulation, CustomError::InsufficientCirculation);
            let mut_reserve = reserves.get_mut(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
            mut_reserve.irma_in_circulation -= irma_amount;
        } else
        if post_other_price_diff <= post_price_diff {
            // msg!("--> Post other price diff is less than or equal to second price diff, 
            //         adjusting other circulation only.");
            // if irma_amount is such that it would reduce discrepancy for other stablecoin more post 
            // adjustment, we can choose to subtract irma_amount from the other_circulation only
            require!(irma_amount <= other_circulation, CustomError::InsufficientCirculation);
            let mut_reserve = reserves.get_mut(other_target).ok_or(CustomError::InvalidQuoteToken)?;
            mut_reserve.irma_in_circulation -= irma_amount;
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
            let mut_reserve = reserves.get_mut(other_target).ok_or(CustomError::InvalidQuoteToken)?;
            mut_reserve.irma_in_circulation -= adjustment_amount.ceil() as u64;
            let mut_reserve = reserves.get_mut(quote_token).ok_or(CustomError::InvalidQuoteToken)?;
            mut_reserve.irma_in_circulation -= irma_amount - adjustment_amount.ceil() as u64;
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
    #[msg("No reserve list.")]
    InvalidReserveList,
    #[msg("Invalid backing symbol.")]
    InvalidBackingSymbol,
    #[msg("Invalid backing address.")]
    InvalidBackingAddress,
}

