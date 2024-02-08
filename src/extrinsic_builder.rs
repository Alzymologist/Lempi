use frame_metadata::v14::RuntimeMetadataV14;

use substrate_constructor::fill_prepare::{MultiAddress, TransactionToFill, TypeContentToFill, TypeToFill, VariantSelector};

pub enum SelectedArea {
    Author,
    Call,
    Extensions,
}

pub struct Builder {
    transaction: TransactionToFill,
    selected_area: SelectedArea,
}

impl Builder {
    pub fn new(metadata: RuntimeMetadataV14) -> Self {
        let transaction = TransactionToFill::init(&mut (), &metadata).unwrap();
        let selected_area = SelectedArea::Call;
        Self {
            transaction,
            selected_area,
        }
    }

    pub fn author(&self) -> &Option<MultiAddress> {
        &self.transaction.author
    }

    pub fn call(&self) -> &TypeToFill {
        &self.transaction.call
    }
}

