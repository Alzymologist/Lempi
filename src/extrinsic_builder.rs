use frame_metadata::{v14::RuntimeMetadataV14, RuntimeMetadata};

use substrate_constructor::fill_prepare::TransactionToFill;

use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::widgets::{layout, RenderArgs, UpdateArgs, Widget, WidgetEvent};

use tokio::sync::broadcast;

use crate::chain;

pub struct ExtrinsicBuilder {
    //transaction: &'a TransactionToFill,
}

impl<'a> ExtrinsicBuilder {
    pub fn new() -> Self {
        Self{
            //transaction,
        }
    }
}

impl Widget for ExtrinsicBuilder {
    /*
    fn process_event(&mut self, event: &WidgetEvent, _args: &mut UpdateArgs) -> bool {
        /*
            match event {
                WidgetEvent::Input(InputEvent::Key(KeyEvent {
                    key: KeyCode::Char(c),
                    ..
                })) => self.text.push(*c),
                WidgetEvent::Input(InputEvent::Key(KeyEvent {
                    key: KeyCode::Enter,
                    ..
                })) => {
                    self.text.push_str("\r\n");
                }
                WidgetEvent::Input(InputEvent::Paste(s)) => {
                    self.text.push_str(&s);
                }
                _ => {}
            }
        */
        true // handled it all
    }
    */

    fn render(&mut self, args: &mut RenderArgs) {
        args.surface.add_change(Change::ClearScreen(AnsiColor::Black.into()));
        //args.surface.add_change(format!("Extrinsic builder!\n\n\nCall:\n{:?}\n\nExtensions:\n{:?}", self.transaction.call, self.transaction.extensions));
    }
}

