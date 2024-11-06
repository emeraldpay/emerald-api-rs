use std::str::FromStr;
use crate::proto::common::ChainRef;

impl FromStr for ChainRef {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("CHAIN_") {
            return ChainRef::from_str(&s[6..]);
        }
        match s.to_lowercase().as_str() {
            "bitcoin" | "btc" | "1" => Ok(ChainRef::ChainBitcoin),
            "ethereum" | "eth" | "100" => Ok(ChainRef::ChainEthereum),
            "ethereum_classic" | "ethereum-classic" | "etc" | "101" => Ok(ChainRef::ChainEthereum),
            "sepolia" | "testnet-sepolia" | "testnet_sepolia" | "10009" => Ok(ChainRef::ChainSepolia),
            _ => Err(()),
        }
    }
}