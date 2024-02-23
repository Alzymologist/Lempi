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
        let mut midscreen = 0;
        let mut cursor_seen = false;
        let (_, ysize) = self.surface.dimensions();
        let midscreen_threshold = ysize / 2;

        self.surface
            .add_change(Change::ClearScreen(AnsiColor::Black.into()));

        self.surface.add_change(card.info);
        self.surface.add_change("\n\r");
        self.surface.add_change("\n\r");
        self.surface.add_change(card.content);

        if let Some(selector) = card.selector {
            self.surface.add_change("\n\r");
            self.surface.add_change("\n\r");
            for (index, item) in selector.list.iter().enumerate() {
                if cursor_seen {
                    midscreen += 1;
                    if midscreen > midscreen_threshold {
                        self.surface.add_change(" + + + ( more )\n\r");
                        break;
                    }
                }
                if selector.index() == index {
                    cursor_seen = true;
                    self.surface
                        .add_change(Change::Attribute(AttributeChange::Background(
                            AnsiColor::White.into(),
                        )));
                }
                self.surface.add_change(item);
                self.surface.add_change("\n\r");
                if selector.index() == index {
                    self.surface
                        .add_change(Change::Attribute(AttributeChange::Background(
                            AnsiColor::Black.into(),
                        )));
                }
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
