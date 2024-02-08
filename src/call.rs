use substrate_constructor::fill_prepare::{TypeContentToFill, TypeToFill, VariantSelector};

use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Surface};

pub struct CallField {
    surface: Surface,
}

impl CallField {
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }

    pub fn render(&mut self, call: &TypeToFill, position: usize) -> &Surface {
        self.surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        let cards = steamroller(call, 0);
        for (depth, card) in cards.into_iter().enumerate() {
            self.render_field(card, position==depth)
        }
        //self.surface.add_change(format!("Call: {:?}\n", call.content));
        //self.surface.add_change(format!("Info: {:?}", call.info));
        &self.surface
    }

    fn render_field(&mut self, card: Card, selected: bool) {
        if selected {
            self.surface.add_change(Change::Attribute(AttributeChange::Background(AnsiColor::White.into())));
        }
        for i in 0..card.indent {
            self.surface.add_change(" | ");
        }
        self.surface.add_change(card.content);
        self.surface.add_change("\n\r");
        if selected {
            self.surface.add_change(Change::Attribute(AttributeChange::Background(AnsiColor::Black.into())));
        }
    }
}

/// Renderable card for single editable field
struct Card {
    content: String,
    indent: usize,
}

impl Card {
    pub fn new(content: String, indent: usize) -> Self {
        Self {
            content,
            indent,
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
        TypeContentToFill::Variant(a) => {
            output.push(Card::new(a.selected.name.clone(), indent));
            for i in &a.selected.fields_to_fill {
                output.append(&mut steamroller(&i.type_to_fill, indent+1));
            }
        },

        inside @ _ => {
            output.push(Card::new(format!("Not implemented: {:?}\n", inside), indent));
        },
    };
    output
}


