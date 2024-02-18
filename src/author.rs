use primitive_types::H256;

use substrate_parser::additional_types::AccountId32;

pub struct Address {
    public: H256,
}

impl Address {
    pub fn from_public(public: H256) -> Self {
        Self{
            public,
        }
    }

    pub fn from_public_hex(public: &str) -> Self {
        Self::from_public(H256(hex::decode(public).unwrap().try_into().unwrap()))
    }

    /*
    pub fn from_private() -> Self {
        Self{}
    }
    */

    fn own_symbol(&self) -> String {
        "+".to_string()
    }

    pub fn public(&self) -> H256 {
        self.public
    }

    pub fn into_account_id32(&self) -> AccountId32 {
        AccountId32(self.public().0)
    }

    pub fn name(&self, ss58: u16) -> String {
        format!("[{}] {}", self.own_symbol(), self.into_account_id32().as_base58(ss58).to_string())
    }
}

pub struct AddressBook {
    content: Vec<Address>
}

impl AddressBook {
    pub fn init() -> Self {
        let mut content = Vec::new();
        // Debuggers built-in addresses
        content.push(Address::from_public_hex("be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"));
        content.push(Address::from_public_hex("fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"));
        content.push(Address::from_public_hex("90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22"));
        content.push(Address::from_public_hex("306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20"));
        Self {
            content
        }
    }

    pub fn authors(&self) -> &Vec<Address> {
        &self.content
    }

    pub fn author_names(&self, ss58: u16) -> Vec<String> {
        self.authors().iter().map(|a| format!("{}", a.name(ss58))).collect()
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
