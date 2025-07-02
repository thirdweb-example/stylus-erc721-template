#![cfg_attr(not(any(feature = "export-abi", test)), no_main)]
extern crate alloc;

// Modules and imports
mod erc721;

use alloy_primitives::{U256, Address};
use stylus_sdk::{
    msg, prelude::*
};
use crate::erc721::{Erc721, Erc721Params, Ownable};

struct StylusNFTParams;
impl Erc721Params for StylusNFTParams {
    const NAME: &'static str = "StylusNFT";
    const SYMBOL: &'static str = "SNFT";

    fn token_uri(token_id: U256) -> String {
        format!("{}{}", "ipfs://base_uri/", token_id) // Update your NFT metadata base URI here
    }
}

sol_storage! {
    #[entrypoint]
    struct StylusNFT {
        #[borrow]
        Erc721<StylusNFTParams> erc721;
        #[borrow]
        Ownable ownable;
    }
}

#[public]
#[inherit(Erc721<StylusNFTParams>, Ownable)]
impl StylusNFT {
    #[constructor]
    pub fn constructor(&mut self, owner: Address) {
        let _ = self.ownable._set_owner(owner);
    }

    pub fn mint(&mut self, to: Address) -> Result<(), String> {
        self.erc721.mint(to)?;
        Ok(())
    }

    pub fn burn(&mut self, token_id: U256) -> Result<(), String> {
        self.erc721.burn(msg::sender(), token_id)?;
        Ok(())
    }

    pub fn total_supply(&self) -> Result<U256, String> {
        Ok(self.erc721.total_supply.get())
    }
}