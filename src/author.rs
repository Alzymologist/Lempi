use substrate_constructor::fill_prepare::MultiAddress;

use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Surface};

pub struct AuthorField {
    surface: Surface,
}

impl AuthorField {
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }

    pub fn render(&mut self, author: &Option<MultiAddress>, _position: usize) -> &Surface {
        self.surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        self.surface.add_change(format!("Author: {:?}", author));
        &self.surface
    }
}

