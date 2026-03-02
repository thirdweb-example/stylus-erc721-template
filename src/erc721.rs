use alloc::{string::String, vec::Vec};
use alloy_primitives::{Address, U256, FixedBytes};
use alloy_sol_types::sol;
use core::marker::PhantomData;
use stylus_sdk::{
    abi::Bytes,
    prelude::*
};

pub trait Erc721Params {
    const NAME: &'static str;
    const SYMBOL: &'static str;
    fn token_uri(token_id: U256) -> String;
}

sol_storage! {
    pub struct Erc721<T> {
        mapping(uint256 => address) owners;
        mapping(address => uint256) balances;
        mapping(uint256 => address) token_approvals;
        mapping(address => mapping(address => bool)) operator_approvals;
        uint256 total_supply;
        PhantomData<T> phantom;
    }

    pub struct Ownable {
        address owner;
    }
}

sol! {
    event Transfer(address indexed from, address indexed to, uint256 indexed token_id);
    event Approval(address indexed owner, address indexed approved, uint256 indexed token_id);
    event ApprovalForAll(address indexed owner, address indexed operator, bool approved);
}

sol_interface! {
    interface IERC721TokenReceiver {
        function onERC721Received(address operator, address from, uint256 token_id, bytes data) external view returns(bytes4);
    }
}

const ERC721_TOKEN_RECEIVER_ID: u32 = 0x150b7a02;

#[public]
impl Ownable {
    pub fn owner(&self) -> Result<Address, String> {
        Ok(self.owner.get())
    }

    pub fn set_owner(&mut self, new_owner: Address) -> Result<(), String> {
        self._check_owner()?;
        self._set_owner(new_owner)?;

        Ok(())
    }
}

impl Ownable {
    pub fn _check_owner(&self) -> Result<(), String> {
        let msg_sender = self.vm().msg_sender();
        let owner = self.owner.get();

        if msg_sender != owner {
            return Err("Not authorized".into());
        }

        Ok(())
    }

    pub fn _set_owner(&mut self, new_owner: Address) -> Result<(), String> {
        if new_owner != Address::ZERO {
            return Err("Zero address".into());
        }

        self.owner.set(new_owner);
        
        Ok(())
    }
}

impl<T: Erc721Params> Erc721<T> {
    fn require_authorized_to_spend(&self, from: Address, token_id: U256) -> Result<(), String> {
        let owner = self.owner_of(token_id)?;
        if from != owner {
            return Err("Not Owner".into());
        }

        if self.vm().msg_sender() == owner {
            return Ok(());
        }

        if self.operator_approvals.getter(owner).get(self.vm().msg_sender()) {
            return Ok(());
        }

        if self.vm().msg_sender() == self.token_approvals.get(token_id) {
            return Ok(());
        }

        return Err("Not approved".into());
    }

    pub fn transfer(&mut self, token_id: U256, from: Address, to: Address) -> Result<(), String> {
        let mut owner = self.owners.setter(token_id);
        let previous_owner = owner.get();
        if previous_owner != from {
            return Err("Not owner".into());
        }
        owner.set(to);

        let mut from_balance = self.balances.setter(from);
        let balance = from_balance.get() - U256::from(1);
        from_balance.set(balance);

        let mut to_balance = self.balances.setter(to);
        let balance = to_balance.get() + U256::from(1);
        to_balance.set(balance);

        self.token_approvals.delete(token_id);
        
        self.vm().log(Transfer { from, to, token_id });
        Ok(())
    }

    fn call_receiver(
        &self,
        token_id: U256,
        from: Address,
        to: Address,
        data: Vec<u8>,
    ) -> Result<(), String> {
        if self.vm().code_size(to) > 0 {
            let sender = self.vm().msg_sender();
            let receiver = IERC721TokenReceiver::new(to);
            let received = receiver
                .on_erc_721_received(self.vm(), Call::new(), sender, from, token_id, data.into())
                .map_err(|_| "ERC721Receiver: low-level call failed")?
                .0;

            if u32::from_be_bytes(received) != ERC721_TOKEN_RECEIVER_ID {
                return Err("Receiver refused".into());
            }
        }
        Ok(())
    }

    pub fn safe_transfer(
        &mut self,
        token_id: U256,
        from: Address,
        to: Address,
        data: Vec<u8>,
    ) -> Result<(), String> {
        self.transfer(token_id, from, to)?;
        self.call_receiver(token_id, from, to, data)
    }

    pub fn mint(&mut self, to: Address) -> Result<(), String> {
        let new_token_id = self.total_supply.get();
        self.total_supply.set(new_token_id + U256::from(1u8));
        self.transfer(new_token_id, Address::default(), to)?;
        Ok(())
    }

    pub fn burn(&mut self, from: Address, token_id: U256) -> Result<(), String> {
        self.transfer(token_id, from, Address::default())?;
        Ok(())
    }
}

#[public]
impl<T: Erc721Params> Erc721<T> {
    pub fn name() -> Result<String, String> {
        Ok(T::NAME.into())
    }

    pub fn symbol() -> Result<String, String> {
        Ok(T::SYMBOL.into())
    }

    #[selector(name = "tokenURI")]
    pub fn token_uri(&self, token_id: U256) -> Result<String, String> {
        self.owner_of(token_id)?;
        Ok(T::token_uri(token_id))
    }

    pub fn balance_of(&self, owner: Address) -> Result<U256, String> {
        Ok(self.balances.get(owner))
    }

    pub fn owner_of(&self, token_id: U256) -> Result<Address, String> {
        let owner = self.owners.get(token_id);
        if owner.is_zero() {
            return Err("Invalid token Id".into());
        }
        Ok(owner)
    }

    #[selector(name = "safeTransferFrom")]
    pub fn safe_transfer_from_with_data(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
        data: Bytes,
    ) -> Result<(), String> {
        if to.is_zero() {
            return Err("Transfer to zero".into());
        }
        self.require_authorized_to_spend(from, token_id)?;
        self.safe_transfer(token_id, from, to, data.to_vec())
    }

    #[selector(name = "safeTransferFrom")]
    pub fn safe_transfer_from(
        &mut self,
        from: Address,
        to: Address,
        token_id: U256,
    ) -> Result<(), String> {
        self.safe_transfer_from_with_data(from, to, token_id, Bytes::default())
    }

    pub fn transfer_from(&mut self, from: Address, to: Address, token_id: U256) -> Result<(), String> {
        if to.is_zero() {
            return Err("Transfer to zero".into());
        }
        self.require_authorized_to_spend(from, token_id)?;
        self.transfer(token_id, from, to)?;
        Ok(())
    }

    pub fn approve(&mut self, approved: Address, token_id: U256) -> Result<(), String> {
        let owner = self.owner_of(token_id)?;

        if self.vm().msg_sender() != owner && !self.operator_approvals.getter(owner).get(self.vm().msg_sender()) {
            return Err("Not approved".into());
        }
        self.token_approvals.insert(token_id, approved);

        self.vm().log(Approval {
            approved,
            owner,
            token_id,
        });
        Ok(())
    }

    pub fn set_approval_for_all(&mut self, operator: Address, approved: bool) -> Result<(), String> {
        let owner = self.vm().msg_sender();
        self.operator_approvals
            .setter(owner)
            .insert(operator, approved);

        self.vm().log(ApprovalForAll {
            owner,
            operator,
            approved,
        });
        Ok(())
    }

    pub fn get_approved(&self, token_id: U256) -> Result<Address, String> {
        Ok(self.token_approvals.get(token_id))
    }

    pub fn is_approved_for_all(&self, owner: Address, operator: Address) -> Result<bool, String> {
        Ok(self.operator_approvals.getter(owner).get(operator))
    }

    pub fn supports_interface(interface: FixedBytes<4>) -> Result<bool, String> {
        let interface_slice_array: [u8; 4] = interface.as_slice().try_into().unwrap();

        if u32::from_be_bytes(interface_slice_array) == 0xffffffff {
            return Ok(false);
        }

        const IERC165: u32 = 0x01ffc9a7;
        const IERC721: u32 = 0x80ac58cd;
        const IERC721_METADATA: u32 = 0x5b5e139f;

        Ok(matches!(u32::from_be_bytes(interface_slice_array), IERC165 | IERC721 | IERC721_METADATA))
    }
}