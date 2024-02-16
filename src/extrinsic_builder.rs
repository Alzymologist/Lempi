use frame_metadata::v14::RuntimeMetadataV14;

use serde_json::{Map, Value};

use substrate_constructor::fill_prepare::{
    EraToFill, PrimitiveToFill, SpecialTypeToFill, TransactionToFill, TypeContentToFill,
    TypeToFill, VariantSelector,
};
use substrate_constructor::try_fill::{TryBytesFill, TryFill};

use crate::author::AddressBook;


pub struct Selector {
    pub list: Vec<String>,
    pub index: usize,
}

pub struct Builder<'a, 'b> {
    address_book: &'b AddressBook,
    base58: u16,
    buffer: String,
    pub details: bool,
    genesis_hash: [u8; 32],
    metadata: &'a RuntimeMetadataV14,
    position: usize,
    selector: usize,
    specs: Map<String, Value>,
    transaction: TransactionToFill,
}

impl<'a, 'b> Builder<'a, 'b> {
    pub fn new(metadata: &'a RuntimeMetadataV14, address_book: &'b AddressBook, genesis_hash: [u8; 32], specs: Map<String, Value>) -> Self {
        let mut transaction = TransactionToFill::init(&mut (), metadata).unwrap();
        let base58 = if let Some(Value::Number(a)) = specs.get("ss58Format") {
            if let Some(b) = a.as_u64() {
                b as u16
            } else {42}
        } else {42};
        Self {
            address_book,
            base58,
            buffer: "".to_owned(),
            details: false,
            genesis_hash,
            metadata,
            position: 0,
            selector: 0,
            specs,
            transaction,
        }
    }

    pub fn call(&self) -> Vec<Card> {
        let mut output = steamroller(&self.transaction.author, 0);
        output.append(&mut steamroller(&self.transaction.call, 0));
        for extension in &self.transaction.extensions {
            output.append(&mut steamroller(&extension, 0));
        }
        output
    }

    pub fn details(&self) -> DetailsCard {
        let buffer = if self.details {
            Some(self.buffer.clone())
        } else {
            None
        };
        let selector = if self.details {
            Some(self.selector)
        } else {
            None
        };
        DetailsCard::new(
            &self.observable_field(),
            buffer,
            selector,
            self.address_book,
        )
    }

    pub fn up(&mut self) {
        if self.details {
            if self.selector > 0 {
                self.selector -= 1;
            }
        } else {
            if self.position > 0 {
                self.position -= 1;
            }
        }
    }

    pub fn down(&mut self) {
        if self.details {
            self.selector += 1;
        } else {
            if self.position < self.call().len() - 1 {
                self.position += 1;
            }
        }
    }

    pub fn left(&mut self) {
        let types = &self.metadata.types;
        match self.modifiable_field().content {
            TypeContentToFill::SpecialType(SpecialTypeToFill::Era(ref mut a)) => a.selector(),
            TypeContentToFill::Variant(ref mut a) => a
                .selector_up::<(), RuntimeMetadataV14>(&mut (), types)
                .unwrap(),
            _ => (),
        };
    }

    pub fn right(&mut self) {
        let types = &self.metadata.types;
        match self.modifiable_field().content {
            TypeContentToFill::SpecialType(SpecialTypeToFill::Era(ref mut a)) => a.selector(),
            TypeContentToFill::Variant(ref mut a) => a
                .selector_down::<(), RuntimeMetadataV14>(&mut (), types)
                .unwrap(),
            _ => (),
        };
    }

    pub fn enter(&mut self) {
        if self.details {
            let buffer = self.buffer.clone();
            let types = &self.metadata.types;
            let selector = self.selector;
            match self.modifiable_field().content {
                TypeContentToFill::ArrayU8(ref mut a) => {
                    a.upd_from_utf8(&buffer);
                }
                TypeContentToFill::Primitive(ref mut a) => match a {
                    PrimitiveToFill::CompactUnsigned(ref mut b) => b.content.upd_from_str(&buffer),
                    PrimitiveToFill::Regular(ref mut b) => b.upd_from_str(&buffer),
                    PrimitiveToFill::Unsigned(ref mut b) => b.content.upd_from_str(&buffer),
                },
                TypeContentToFill::SequenceU8(ref mut a) => {
                    a.upd_from_utf8(&buffer);
                }
                TypeContentToFill::Variant(ref mut a) => {
                    match VariantSelector::new_at::<(), RuntimeMetadataV14>(
                        &a.available_variants,
                        &mut (),
                        types,
                        selector,
                    ) {
                        Ok(b) => *a = b,
                        _ => (),
                    }
                }
                _ => {}
            }

            self.buffer = "".to_string();
            self.selector = 0;
            self.details = false;
        } else {
            self.buffer = "".to_string();
            self.selector = 0;
            self.details = true;
        }
    }

    pub fn input(&mut self, c: char) {
        self.buffer.push(c);
        /*
        if self.details {
            match self.modifiable_field().content {
                TypeContentToFill::Sequence(ref mut a) => {
                    match a.content {
                        SetInProgress::U8(ref mut v) => v.push(c as u8),
                        SetInProgress::Regular(ref mut v) => (),
                    }
                },
                _ => {},
            }
        }*/
    }

    pub fn backspace(&mut self) {
        let _ = self.buffer.pop();
        /*
        if self.details {
            match self.modifiable_field().content {
                TypeContentToFill::Sequence(ref mut a) => {
                    match a.content {
                        SetInProgress::U8(ref mut v) => {let _ = v.pop();},
                        SetInProgress::Regular(ref mut v) => {let _ = v.pop();},
                    }
                },
                _ => {},
            }
        }
        */
    }

    pub fn paste(&mut self, s: String) {
        self.buffer.push_str(&s);
        /*
        if self.details {
            match self.modifiable_field().content {
                TypeContentToFill::Sequence(ref mut a) => {
                    match a.content {
                        SetInProgress::U8(ref mut v) => v.append(&mut (s.as_bytes().to_vec())),
                        SetInProgress::Regular(ref mut v) => (),
                    }
                },
                _ => {},
            }
        }*/
    }

    pub fn position(&self) -> usize {
        self.position
    }

    fn observable_field(&self) -> &TypeToFill {
        let mut position = self.position;
        match peek(&self.transaction.author, position) {
            Peeker::Done(a) => return a,
            Peeker::Depth(a) => {
                position = a;
                match peek(&self.transaction.call, position) {
                    Peeker::Done(a) => return a,
                    Peeker::Depth(b) => {
                        position = b;
                        for extension in &self.transaction.extensions {
                            match peek(extension, position) {
                                Peeker::Done(a) => return a,
                                Peeker::Depth(c) => {
                                    position = c;
                                }
                            }
                        }
                    }
                }
            }
        }

        panic!("diver reached bottom of pool, rip");
    }

    fn modifiable_field(&mut self) -> &mut TypeToFill {
        let mut position = self.position;
        match peek(&self.transaction.author, position) {
            Peeker::Done(_) => return dive_hard(&mut self.transaction.author, position),
            Peeker::Depth(a) => {
                position = a;
                match peek(&self.transaction.call, position) {
                    Peeker::Done(_) => return dive_hard(&mut self.transaction.call, position),
                    Peeker::Depth(b) => {
                        position = b;
                        for extension in &mut self.transaction.extensions {
                            match peek(extension, position) {
                                Peeker::Done(_) => return dive_hard(extension, position),
                                Peeker::Depth(c) => {
                                    position = c;
                                }
                            }
                        }
                    }
                }
            }
        }

        panic!("diver reached bottom of pool, rip");
    }

    pub fn autofill(&mut self) {
        // TODO

    }
}

/// Renderable card for single editable field
pub struct Card {
    pub content: String,
    pub indent: usize,
}

impl Card {
    pub fn new(content: String, indent: usize) -> Self {
        Self { content, indent }
    }
}


pub struct DetailsCard {
    pub content: String,
    pub info: String,
    pub buffer: Option<String>,
    pub selector: Option<Selector>,
}

impl DetailsCard {
    pub fn new(
        input: &TypeToFill,
        buffer: Option<String>,
        selector_index: Option<usize>,
        address_book: &AddressBook,
    ) -> Self {
        let info = input
            .info
            .iter()
            .map(|a| a.docs.clone())
            .collect::<Vec<String>>()
            .join(" ~ ");
        let mut selector = None;
        let content = match &input.content {
            TypeContentToFill::ArrayU8(a) => {
                format!(
                    "Value: {}\r\n\r\nString: {}\r\n\r\nLength: {}",
                    hex::encode(a.content.to_vec()),
                    String::from_utf8(a.content.to_vec()).unwrap_or("not UTF8".to_string()),
                    a.len
                )
            }

            TypeContentToFill::SequenceU8(a) => {
                format!(
                    "Value: {}\r\n\r\nString: {}",
                    hex::encode(a.content.to_vec()),
                    String::from_utf8(a.content.to_vec()).unwrap_or("not UTF8".to_string())
                )
            }
            TypeContentToFill::SpecialType(SpecialTypeToFill::AccountId32(a)) => {
                selector = Some(Selector {list: address_book.author_names(), index: 0});
                format!("{:?}", a)
            }
            TypeContentToFill::Variant(a) => {
                if let Some(index) = selector_index {
                    let mut list = Vec::new();
                    for variant in &a.available_variants {
                        list.push(variant.name.clone());
                    }
                    selector = Some(Selector { list, index });
                }
                format!(
                    "{}: {}",
                    a.selected.name,
                    a.selected.docs.replace("\n", "\r\n")
                )
            }
            inside @ _ => format!("{:?}", inside),
        };
        DetailsCard {
            content,
            info,
            buffer,
            selector,
        }
    }
}

/// Flatten whole type into renderable cards
///
/// No depth counter implemented as this should be guaranteed elsewhere?
/// in metadata or its parser?
///
/// Either way, if this crashes, no biggie
fn steamroller(input: &TypeToFill, indent: usize) -> Vec<Card> {
    let mut output = Vec::new();
    match &input.content {
        TypeContentToFill::ArrayU8(a) => {
            output.push(Card::new(format!("0x{}", hex::encode(&a.content)), indent));
        }
        TypeContentToFill::Composite(a) => {
            for i in a {
                output.append(&mut steamroller(&i.type_to_fill, indent));
            }
        }
        TypeContentToFill::Primitive(PrimitiveToFill::CompactUnsigned(a)) => {
            output.push(Card::new(
                format!("{:?}: {:?}", a.specialty, a.content),
                indent,
            ));
        }
        TypeContentToFill::Primitive(PrimitiveToFill::Regular(a)) => {
            output.push(Card::new(format!("{:?}", a), indent));
        }
        TypeContentToFill::Primitive(PrimitiveToFill::Unsigned(a)) => {
            output.push(Card::new(
                format!("{:?}: {:?}", a.specialty, a.content),
                indent,
            ));
        }
        TypeContentToFill::SequenceU8(a) => {
            output.push(Card::new(format!("0x{}", hex::encode(&a.content)), indent));
        }
        TypeContentToFill::SpecialType(SpecialTypeToFill::AccountId32(None)) => {
            output.push(Card::new("AccountId32".to_string(), indent));
        }
        TypeContentToFill::SpecialType(SpecialTypeToFill::AccountId32(Some(a))) => {
            output.push(Card::new(a.as_base58(42), indent)); // TODO
        }

        TypeContentToFill::SpecialType(SpecialTypeToFill::Era(EraToFill::Immortal)) => {
            output.push(Card::new("Immortal".to_string(), indent));
        }
        TypeContentToFill::SpecialType(SpecialTypeToFill::Era(EraToFill::Mortal {
            phase,
            period,
        })) => {
            output.push(Card::new(
                format!("Phase: {:?} Period: {:?}", phase, period),
                indent,
            ));
        }
        TypeContentToFill::Tuple(a) => {
            for i in a {
                output.append(&mut steamroller(&i, indent));
            }
        }
        TypeContentToFill::Variant(a) => {
            output.push(Card::new(a.selected.name.clone(), indent));
            for i in &a.selected.fields_to_fill {
                output.append(&mut steamroller(&i.type_to_fill, indent + 1));
            }
        }
        TypeContentToFill::VariantEmpty => {}

        inside @ _ => {
            output.push(Card::new(format!("Not implemented: {:?}", inside), indent));
        }
    };
    output
}

enum Peeker<'a> {
    Depth(usize),
    Done(&'a TypeToFill),
}

/// Extract type at given depth
fn peek<'a>(input: &'a TypeToFill, position: usize) -> Peeker<'a> {
    let mut depth = position;
    match input.content {
        TypeContentToFill::Composite(ref a) => {
            for i in a {
                match peek(&i.type_to_fill, depth) {
                    Peeker::Depth(a) => depth = a,
                    Peeker::Done(a) => return Peeker::Done(a),
                }
            }
        }
        TypeContentToFill::Tuple(ref a) => {
            for i in a {
                match peek(&i, depth) {
                    Peeker::Depth(a) => depth = a,
                    Peeker::Done(a) => return Peeker::Done(a),
                }
            }
        }
        TypeContentToFill::Variant(ref a) => {
            if position == 0 {
                return Peeker::Done(input);
            }
            depth -= 1;
            for i in &a.selected.fields_to_fill {
                match peek(&i.type_to_fill, depth) {
                    Peeker::Depth(a) => depth = a,
                    Peeker::Done(a) => return Peeker::Done(a),
                }
            }
        }
        TypeContentToFill::VariantEmpty => {}
        _ => {
            if position == 0 {
                return Peeker::Done(input);
            }
            depth -= 1;
        }
    };

    Peeker::Depth(depth)
}

enum Diver<'a> {
    Depth(usize),
    Done(&'a mut TypeToFill),
}

/// Extract type at given depth
fn dive<'a>(input: &'a mut TypeToFill, position: usize) -> Diver<'a> {
    let mut depth = position;

    //Pass twice because of ref mut limitations
    match input.content {
        TypeContentToFill::Composite(_) => {}
        TypeContentToFill::Tuple(_) => {}
        TypeContentToFill::Variant(_) => {
            if position == 0 {
                return Diver::Done(input);
            }
        }
        TypeContentToFill::VariantEmpty => {}
        _ => {
            if position == 0 {
                return Diver::Done(input);
            };
        }
    };

    match input.content {
        TypeContentToFill::Composite(ref mut a) => {
            for i in a {
                match dive(&mut i.type_to_fill, depth) {
                    Diver::Depth(a) => depth = a,
                    Diver::Done(a) => return Diver::Done(a),
                }
            }
        }
        TypeContentToFill::Tuple(ref mut a) => {
            for i in a {
                match dive(i, depth) {
                    Diver::Depth(a) => depth = a,
                    Diver::Done(a) => return Diver::Done(a),
                }
            }
        }
        TypeContentToFill::Variant(ref mut a) => {
            depth -= 1;
            for i in &mut a.selected.fields_to_fill {
                match dive(&mut i.type_to_fill, depth) {
                    Diver::Depth(a) => depth = a,
                    Diver::Done(a) => return Diver::Done(a),
                }
            }
        }
        TypeContentToFill::VariantEmpty => {}
        _ => {
            depth -= 1;
        }
    };

    Diver::Depth(depth)
}

fn dive_hard<'a>(input: &'a mut TypeToFill, position: usize) -> &mut TypeToFill {
    match dive(input, position) {
        Diver::Done(a) => a,
        _ => panic!("diver reached bottom of the pool!"),
    }
}
