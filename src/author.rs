use primitive_types::H256;

use sp_core::{sr25519, Pair};

use substrate_parser::additional_types::AccountId32;

#[derive(Debug)]
pub enum Error {
    DerivationFailed(String),
}

pub enum Address {
    Public(H256),
    Pair(sr25519::Pair),
}

impl Address {
    pub fn from_public(public: H256) -> Self {
        Self::Public(public)
    }

    pub fn from_public_hex(public: &str) -> Self {
        Self::from_public(H256(hex::decode(public).unwrap().try_into().unwrap()))
    }

    pub fn from_derivation(full_address: &str) -> Result<Self, Error> {
        match sr25519::Pair::from_string(full_address, None) {
            Ok(a) => Ok(Self::Pair(a)),
            Err(e) => Err(Error::DerivationFailed(e.to_string())),
        }
    }

    /*
    pub fn from_private() -> Self {
        Self{}
    }
    */

    fn own_symbol(&self) -> String {
        match self {
            Address::Public(_) => "-".to_string(),
            Address::Pair(_) => "+".to_string(),
        }
    }

    pub fn public(&self) -> H256 {
        match self {
            Address::Public(a) => *a,
            Address::Pair(a) => a.public().into(),
        }
    }

    pub fn into_account_id32(&self) -> AccountId32 {
        AccountId32(self.public().0)
    }

    pub fn name(&self, ss58: u16) -> String {
        format!(
            "[{}] {}",
            self.own_symbol(),
            self.into_account_id32().as_base58(ss58).to_string()
        )
    }

    pub fn sign(&self, input: &[u8]) -> Option<sr25519::Signature> {
        match self {
            Address::Public(_) => None,
            Address::Pair(a) => Some(a.sign(input)),
        }
    }
}

pub struct AddressBook {
    content: Vec<Address>,
    ss58: u16,
}

impl AddressBook {
    pub fn init(ss58: u16) -> Self {
        let mut content = Vec::new();
        // Debuggers built-in addresses
        content.push(Address::from_derivation("").unwrap());
        content.push(Address::from_public_hex(
            "be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f",
        ));
        content.push(Address::from_derivation("//Alice").unwrap());
        content.push(Address::from_public_hex(
            "fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e",
        ));
        content.push(Address::from_derivation("//Bob").unwrap());
        content.push(Address::from_derivation("//Charlie").unwrap());
        content.push(Address::from_public_hex(
            "306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20",
        ));
        content.push(Address::from_derivation("//Fred").unwrap());
        Self { content, ss58 }
    }

    pub fn authors(&self) -> &Vec<Address> {
        &self.content
    }

    pub fn author_names(&self) -> Vec<String> {
        self.authors()
            .iter()
            .map(|a| format!("{}", a.name(self.ss58)))
            .collect()
    }

    pub fn public(&self, index: usize) -> Option<H256> {
        match self.authors().get(index) {
            Some(a) => Some(a.public()),
            None => None,
        }
    }

    pub fn account_id32(&self, index: usize) -> Option<AccountId32> {
        match self.authors().get(index) {
            Some(a) => Some(a.into_account_id32()),
            None => None,
        }
    }
}
