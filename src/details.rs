use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Surface};

use crate::extrinsic_builder::DetailsCard;

pub struct Details {
    surface: Surface,
}

impl Details {
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }

    pub fn render(&mut self, card: DetailsCard) -> &Surface {
        self.surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        self.surface.add_change(card.info);
        self.surface.add_change("\n\r");
        self.surface.add_change("\n\r");
        self.surface.add_change(card.content);
        &self.surface
    }
}

