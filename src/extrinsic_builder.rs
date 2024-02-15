use frame_metadata::v14::RuntimeMetadataV14;

use substrate_constructor::fill_prepare::{PrimitiveToFill, SetInProgress, SpecialTypeToFill, TransactionToFill, TypeContentToFill, TypeToFill, VariantSelector};

use substrate_parser::additional_types::Era;

pub enum SelectedArea {
    Author,
    Call,
    Extensions,
}

pub struct Builder {
    call_position: usize,
    details: bool,
    metadata: RuntimeMetadataV14,
    transaction: TransactionToFill,
    selected_area: SelectedArea,
}

impl Builder {
    pub fn new(metadata: RuntimeMetadataV14) -> Self {
        let mut transaction = TransactionToFill::init(&mut (), &metadata).unwrap();
        let selected_area = SelectedArea::Call; 
        Self {
            call_position: 0,
            details: false,
            metadata,
            transaction,
            selected_area,
        }
    }

    pub fn author(&self) -> &Option<MultiAddress> {
        &self.transaction.author
    }

    pub fn call(&self) -> Vec<Card> {
        steamroller(&self.transaction.call, 0)
    }

    pub fn extensions(&self) -> Vec<Card> {
        let mut output = Vec::new();
        for extension in &self.transaction.extensions {
            output.append(&mut steamroller(&extension, 0));
        }
        output
    }

    pub fn details(&self, position: usize) -> DetailsCard {
        match peek(&self.transaction.call, position) {
            Peeker::Done(a) => DetailsCard::new(a),
            Peeker::Depth(e) => panic!("too deep, sempai!: {}", e),
        }
    }

    pub fn up(&mut self) {
        if self.call_position > 0 { self.call_position -= 1; }
    }
    
    pub fn down(&mut self) {
        if self.call_position < steamroller(&self.transaction.call, 0).len() { self.call_position += 1; }
    }

    pub fn left(&mut self) {
        let mut item = match dive(&mut self.transaction.call, self.call_position) {
            Diver::Done(a) => a,
            _ => panic!("diver reached bottom of pool, rip"),
        };

        match item.content {
            TypeContentToFill::Variant(ref mut a) => a.selector_up::<(), RuntimeMetadataV14>(&mut (), &self.metadata.types).unwrap(),
            _ => (),
        };
    }

    pub fn right(&mut self) {
        let mut item = match dive(&mut self.transaction.call, self.call_position) {
            Diver::Done(a) => a,
            _ => panic!("diver reached bottom of pool, rip"),
        };

        match item.content {
            TypeContentToFill::Variant(ref mut a) => a.selector_down::<(), RuntimeMetadataV14>(&mut (), &self.metadata.types).unwrap(),
            _ => (),
        };
    }

    pub fn enter(&mut self) {
        self.details = !self.details;
        /*
        let mut item = match dive(&mut self.transaction.call, self.call_position) {
            Diver::Done(a) => a,
            _ => panic!("diver reached bottom of pool, rip"),
        };

        match item.content {
            TypeContentToFill::Variant(ref mut a) => (),
            _ => (),
        };*/
    }
    
    pub fn input(&mut self, c: char) {
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
        }
    }

    pub fn backspace(&mut self) {
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

    }

    pub fn paste(&mut self, s: String) {
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
        }

    }

    pub fn position(&self) -> usize {
        self.call_position
    }

    fn observable_field(&mut self) -> &TypeToFill {
        match peek(&mut self.transaction.call, self.call_position) {
            Peeker::Done(a) => a,
            _ => panic!("diver reached bottom of pool, rip"),
        }
    }

    fn modifiable_field(&mut self) -> &mut TypeToFill {
        match dive(&mut self.transaction.call, self.call_position) {
            Diver::Done(a) => a,
            _ => panic!("diver reached bottom of pool, rip"),
        }
    }
}

/// Renderable card for single editable field
pub struct Card {
    pub content: String,
    pub indent: usize,
}

impl Card {
    pub fn new(content: String, indent: usize) -> Self {
        Self {
            content,
            indent,
        }
    }
}

pub struct DetailsCard {
    pub content: String,
    pub info: String,
}

impl DetailsCard {
    pub fn new(input: &TypeToFill) -> Self {
        let info = input.info.iter().map(|a| a.docs.clone()).collect::<Vec<String>>().join(" ~ ");
        // TODO
        let content = match &input.content {
            TypeContentToFill::Sequence(a) => {
                match &a.content {
                    SetInProgress::U8(v) => format!("Value: {}\r\n\r\nString: {}", hex::encode(v), String::from_utf8(v.to_vec()).unwrap_or("not UTF8".to_string())),
                    SetInProgress::Regular(v) => format!("{:?}", v),
                }
            },
            TypeContentToFill::Variant(a) => {
                format!("{}: {}", a.selected.name, a.selected.docs.replace("\n", "\r\n"))
            },
            inside @ _ => format!("{:?}", inside),
        };
        DetailsCard{content, info}
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
        TypeContentToFill::Array(a) => {
            output.push(Card::new(format!("{:?}", a.sequence.content), indent));
        }
        TypeContentToFill::Composite(a) => {
            for i in a {
                output.append(&mut steamroller(&i.type_to_fill, indent));
            }
        },
        TypeContentToFill::Primitive(PrimitiveToFill::CompactUnsigned(a)) => {
            output.push(Card::new(format!("{:?}: {:?}", a.specialty, a.content), indent));
        }
        TypeContentToFill::Primitive(PrimitiveToFill::Unsigned(a)) => {
            output.push(Card::new(format!("{:?}: {:?}", a.specialty, a.content), indent));
        }
        TypeContentToFill::Sequence(a) => {
            output.push(Card::new(format!("{:?}", a.content), indent));
        }
        TypeContentToFill::SpecialType(SpecialTypeToFill::Era(Era::Immortal)) => {
            output.push(Card::new("Immortal".to_string(), indent));
        }
        TypeContentToFill::SpecialType(SpecialTypeToFill::Era(Era::Mortal(phase, period))) => {
            output.push(Card::new(format!("Phase: {} Period: {}", phase, period), indent));
        }
        TypeContentToFill::Variant(a) => {
            output.push(Card::new(a.selected.name.clone(), indent));
            for i in &a.selected.fields_to_fill {
                output.append(&mut steamroller(&i.type_to_fill, indent+1));
            }
        },

        inside @ _ => {
            output.push(Card::new(format!("Not implemented: {:?}", inside), indent));
        },
    };
    output
}

enum Peeker<'a> {
    Depth(usize),
    Done(&'a TypeToFill),
}

/// Extract type at given depth
fn peek<'a>(input: &'a TypeToFill, position: usize) -> Peeker<'a> {
    if position == 0 { return Peeker::Done(input) }
    let mut depth = position - 1;
    match input.content {
        TypeContentToFill::Variant(ref a) => {
            for i in &a.selected.fields_to_fill {
                match peek(&i.type_to_fill, depth) {
                    Peeker::Depth(a) => depth = a,
                    Peeker::Done(a) => return Peeker::Done(a),
                }
            }
        },
        _ => {
        },
    };

    Peeker::Depth(depth)
}


enum Diver<'a> {
    Depth(usize),
    Done(&'a mut TypeToFill)
}

/// Extract type at given depth
fn dive<'a>(input: &'a mut TypeToFill, position: usize) -> Diver<'a> {
    if position == 0 { return Diver::Done(input) }
    let mut depth = position - 1;
    match input.content {
        TypeContentToFill::Variant(ref mut a) => {
            for i in &mut a.selected.fields_to_fill {
                match dive(&mut i.type_to_fill, depth) {
                    Diver::Depth(a) => depth = a,
                    Diver::Done(a) => return Diver::Done(a),
                }
            }
        },
        _ => {
        },
    };

    Diver::Depth(depth)
}

