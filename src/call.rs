use substrate_constructor::fill_prepare::{TypeContentToFill, TypeToFill, VariantSelector};

use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Surface};

use crate::extrinsic_builder::Card;

pub struct CallField {
    surface: Surface,
}

impl CallField {
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }

    pub fn render(&mut self, cards: Vec<Card>, position: &usize) -> &Surface {
        self.surface
            .add_change(Change::ClearScreen(AnsiColor::Black.into()));
        for (depth, card) in cards.into_iter().enumerate() {
            self.render_field(card, *position == depth)
        }
        &self.surface
    }

    fn render_field(&mut self, card: Card, selected: bool) {
        if selected {
            self.surface
                .add_change(Change::Attribute(AttributeChange::Background(
                    AnsiColor::White.into(),
                )));
        }
        for i in 0..card.indent {
            self.surface.add_change(" | ");
        }
        self.surface.add_change(card.content);
        self.surface.add_change("\r\n");
        if selected {
            self.surface
                .add_change(Change::Attribute(AttributeChange::Background(
                    AnsiColor::Black.into(),
                )));
        }
    }
}
