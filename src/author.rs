use substrate_constructor::fill_prepare::MultiAddress;

use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::widgets::{layout, RenderArgs, UpdateArgs, Widget, WidgetEvent};

pub struct AuthorField<'a> {
    author: &'a Option<MultiAddress>,
}

impl<'a> AuthorField<'a> {
    pub fn new(author: &'a mut Option<MultiAddress>) -> Self {
        Self{author}
    }
}

impl<'a> Widget for AuthorField<'a> {
    fn render(&mut self, args: &mut RenderArgs) {
        args.surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        args.surface.add_change(format!("Author:\n{:?}", self.author));
    }

}

