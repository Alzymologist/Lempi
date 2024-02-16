use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, CursorVisibility, Surface};

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
       
        if let Some(selector) = card.selector {
            self.surface.add_change("\n\r");
            self.surface.add_change("\n\r");
            for (index, item) in selector.list.iter().enumerate() {
                if selector.index == index {self.surface.add_change(Change::Attribute(AttributeChange::Background(AnsiColor::White.into())));}
                self.surface.add_change(item);
                self.surface.add_change("\n\r");
                if selector.index == index {self.surface.add_change(Change::Attribute(AttributeChange::Background(AnsiColor::Black.into())));}
            }
        }

        if let Some(buffer) = card.buffer {    
            self.surface.add_change("\n\r");
            self.surface.add_change("\n\r");
            self.surface.add_change("New value >");
            self.surface.add_change(buffer.clone());
        }

        &self.surface
    }
}

