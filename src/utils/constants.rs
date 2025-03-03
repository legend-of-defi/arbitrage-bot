use alloy::{
    primitives::{address, Address, U256},
    uint,
};

pub const ETHER: U256 = uint!(1_000_000_000_000_000_000_U256);
pub const GWEI: U256 = uint!(1_000_000_000_U256);
pub const WETH: Address = address!("0x4200000000000000000000000000000000000006");
pub const UNISWAP_V2_BATCH_QUERY_ADDRESS: Address =
    address!("0x72D6545d3F45F20754F66a2B99fc1A4D75BFEf5c");
