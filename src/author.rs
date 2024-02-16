use substrate_parser::additional_types::AccountId32;

pub struct Author {
    
}

impl Author {
    pub fn into_account_id32(&self) -> AccountId32 {
        AccountId32([0u8; 32])
    }
}

pub struct AddressBook {

}

impl AddressBook {
    pub fn init() -> Self {
        Self {}
    }

    pub fn authors(&self) -> Vec<Author> {
        vec![Author{}]
    }
}

