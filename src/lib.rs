#![cfg_attr(not(any(feature = "export-abi", test)), no_main)]
extern crate alloc;

// Modules and imports
mod erc721;

use alloy_primitives::{U256, Address};
use stylus_sdk::{
    msg, prelude::*
};
use crate::erc721::{Erc721, Erc721Params, Erc721Error};

struct StylusNFTParams;
impl Erc721Params for StylusNFTParams {
    const NAME: &'static str = "StylusNFT";
    const SYMBOL: &'static str = "SNFT";

    fn token_uri(token_id: U256) -> String {
        format!("{}{}{}", "https://my-nft-metadata.com/", token_id, ".json")
    }
}

sol_storage! {
    #[entrypoint]
    struct StylusNFT {
        #[borrow]
        Erc721<StylusNFTParams> erc721;
    }
}

#[public]
#[inherit(Erc721<StylusNFTParams>)]
impl StylusNFT {
    pub fn mint(&mut self) -> Result<(), Erc721Error> {
        let minter = msg::sender();
        self.erc721.mint(minter)?;
        Ok(())
    }

    pub fn mint_to(&mut self, to: Address) -> Result<(), Erc721Error> {
        self.erc721.mint(to)?;
        Ok(())
    }

    pub fn burn(&mut self, token_id: U256) -> Result<(), Erc721Error> {
        self.erc721.burn(msg::sender(), token_id)?;
        Ok(())
    }

    pub fn total_supply(&mut self) -> Result<U256, Erc721Error> {
        Ok(self.erc721.total_supply.get())
    }
}