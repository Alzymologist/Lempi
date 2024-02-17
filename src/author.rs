use substrate_parser::additional_types::AccountId32;

pub struct Author {
//    public: H256,
}

impl Author {
    pub fn from_public() -> Self {
        Self{}
    }

    pub fn from_private() -> Self {
        Self{}
    }

    pub fn into_account_id32(&self) -> AccountId32 {
        AccountId32([0u8; 32])
    }

    pub fn name(&self) -> String {
        "Pizdec".to_string()
    }
}

pub struct AddressBook {}

impl AddressBook {
    pub fn init() -> Self {
        Self {}
    }

    pub fn authors(&self) -> Vec<Author> {
        vec![Author {}]
    }

    pub fn author_names(&self) -> Vec<String> {
        vec!["Placeholder 1".to_string(), "Placeholcer 2".to_string()]
    }
    
}
