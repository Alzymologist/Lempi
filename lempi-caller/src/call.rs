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
        let mut midscreen = 0;
        let mut cursor_seen = false;
        let (_, ysize) = self.surface.dimensions();
        let midscreen_threshold = ysize / 2;

        self.surface
            .add_change(Change::ClearScreen(AnsiColor::Black.into()));
        for (depth, card) in cards.into_iter().enumerate() {
            if *position == depth {
                cursor_seen = true;
            }
            if cursor_seen {
                midscreen += 1;
                if midscreen > midscreen_threshold {
                    self.surface.add_change(" + + + ( more )\n\r");
                    break;
                }
            }
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
