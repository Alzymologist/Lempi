use termwiz::color::AnsiColor;
use termwiz::surface::Change;
use termwiz::widgets::{layout, RenderArgs, Widget};

use tokio::sync::broadcast;

use crate::chain::plop;

pub struct BlockLine {
    hash_rx: broadcast::Receiver<String>,
    hash: String,
}

impl BlockLine {
    pub fn new(hash_rx: broadcast::Receiver<String>) -> Self {
        let hash = "".to_string();
        Self { hash_rx, hash }
    }
}

impl Widget for BlockLine {
    fn render(&mut self, args: &mut RenderArgs) {
        args.surface
            .add_change(Change::ClearScreen(AnsiColor::Grey.into()));
        // that's right, it's updated only on other events. Maybe this should go into events
        // handler?
        if let Some(a) = plop(&mut self.hash_rx) {
            self.hash = a
        }
        args.surface.add_change(format!("Last block: {}", self.hash));
    }

    fn get_size_constraints(&self) -> layout::Constraints {
        let mut c = layout::Constraints::default();
        c.set_fixed_height(1);
        c.set_valign(layout::VerticalAlignment::Bottom);
        c
    }
}
