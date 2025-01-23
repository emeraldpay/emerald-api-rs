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

impl ChainRef {

    ///
    /// A short code for the blockchain (ex. `BTC` for Bitcoin, etc.)
    pub fn code(&self) -> String {
        match self {
            ChainRef::ChainUnspecified => "UNSPECIFIED".to_string(),
            ChainRef::ChainBitcoin => "BTC".to_string(),
            ChainRef::ChainEthereum => "ETH".to_string(),
            ChainRef::ChainEthereumClassic => "ETC".to_string(),
            ChainRef::ChainFantom => "FTM".to_string(),
            ChainRef::ChainMatic => "MATIC".to_string(),
            ChainRef::ChainRsk => "RSK".to_string(),
            ChainRef::ChainMorden => "MORDEN".to_string(),
            ChainRef::ChainKovan => "KOVAN".to_string(),
            ChainRef::ChainTestnetBitcoin => "TESTNET_BITCOIN".to_string(),
            ChainRef::ChainGoerli => "GOERLI".to_string(),
            ChainRef::ChainRopsten => "ROPSTEN".to_string(),
            ChainRef::ChainRinkeby => "RINKEBY".to_string(),
            ChainRef::ChainHolesky => "HOLESKY".to_string(),
            ChainRef::ChainSepolia => "SEPOLIA".to_string(),
        }
    }

    ///
    /// A full name for the blockchain (ex. `Bitcoin` for Bitcoin, etc.)
    pub fn full_name(&self) -> String {
        match self {
            ChainRef::ChainUnspecified => "Unspecified".to_string(),
            ChainRef::ChainBitcoin => "Bitcoin".to_string(),
            ChainRef::ChainEthereum => "Ethereum".to_string(),
            ChainRef::ChainEthereumClassic => "Ethereum Classic".to_string(),
            ChainRef::ChainFantom => "Fantom".to_string(),
            ChainRef::ChainMatic => "Matic".to_string(),
            ChainRef::ChainRsk => "Bitcoin RSK".to_string(),
            ChainRef::ChainMorden => "Morden".to_string(),
            ChainRef::ChainKovan => "Kovan".to_string(),
            ChainRef::ChainTestnetBitcoin => "Bitcoin Testnet".to_string(),
            ChainRef::ChainGoerli => "Goerli Testnet".to_string(),
            ChainRef::ChainRopsten => "Ropsten Testnet".to_string(),
            ChainRef::ChainRinkeby => "Rinkeby Testnet".to_string(),
            ChainRef::ChainHolesky => "Holesky Testnet".to_string(),
            ChainRef::ChainSepolia => "Sepolia Testnet".to_string(),
        }
    }
}

pub enum BlockchainType {
    Bitcoin,
    Ethereum,
}

impl TryFrom<ChainRef> for BlockchainType {
    type Error = ();

    fn try_from(value: ChainRef) -> Result<Self, Self::Error> {
        let t = match value {
            ChainRef::ChainBitcoin |
            ChainRef::ChainTestnetBitcoin
                => BlockchainType::Bitcoin,

            ChainRef::ChainEthereum |
            ChainRef::ChainEthereumClassic |
            ChainRef::ChainMatic |
            ChainRef::ChainFantom |
            ChainRef::ChainRsk |
            ChainRef::ChainKovan |
            ChainRef::ChainMorden |
            ChainRef::ChainGoerli |
            ChainRef::ChainRinkeby |
            ChainRef::ChainRopsten |
            ChainRef::ChainHolesky |
            ChainRef::ChainSepolia
                => BlockchainType::Ethereum,

            ChainRef::ChainUnspecified => return Err(()),
        };
        Ok(t)
    }
}