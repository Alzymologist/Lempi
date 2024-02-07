use substrate_constructor::fill_prepare::TransactionToFill;


use termwiz::caps::Capabilities;
use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{new_terminal, Terminal};
use termwiz::widgets::{Ui, WidgetEvent};
use termwiz::Error;

use tokio::sync::mpsc;

mod chain;

mod root_screen;
use root_screen::MainScreen;

mod block;
use block::BlockLine;

mod extrinsic_builder;
use extrinsic_builder::ExtrinsicBuilder;

mod author;
use author::AuthorField;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let caps = Capabilities::new_from_env()?;

    let terminal = new_terminal(caps)?;

    let mut buf = BufferedTerminal::new(terminal)?;
    /*
        let mut block = Surface::new(5, 5);
        block.add_change(Change::ClearScreen(AnsiColor::Blue.into()));
        buf.draw_from_screen(&block, 10, 10);

        buf.add_change(Change::Attribute(AttributeChange::Foreground(
            AnsiColor::Maroon.into(),
        )));


        buf.add_change("Hello world\r\n");
        buf.add_change(Change::Attribute(AttributeChange::Foreground(
            AnsiColor::Red.into(),
        )));
        buf.add_change("and in red here\r\n");
        buf.add_change(Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(20),
        });
    */

    buf.flush()?;
    buf.terminal().set_raw_mode()?;

    let (mut block_hash_rx, mut block_rx) = chain::block_watch();

    let some_block = block_rx.recv().await.unwrap();
    let metadata = chain::get_metadata(&some_block).await;

    let transaction = TransactionToFill::init(&mut (), &metadata).unwrap();
//    let mut transaction1 = transaction.clone();
    let mut author = None;

    let mut ui = Ui::new();
    let root_id = ui.set_root(MainScreen::new());

    let extrinsic_builder_id = ui.add_child(root_id, ExtrinsicBuilder::new());
    ui.add_child(extrinsic_builder_id, AuthorField::new(&mut author));
    ui.add_child(root_id, BlockLine::new(block_hash_rx));

    loop {
        ui.process_event_queue()?;

        if ui.render_to_screen(&mut buf)? {
            continue;
        }
        buf.flush()?;

        match buf.terminal().poll_input(None) {
            Ok(Some(input)) => match input {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => {
                    buf.add_change(Change::ClearScreen(AnsiColor::Black.into()));
                    buf.flush()?;
                    break;
                },
                input @ _ => {
                    ui.queue_event(WidgetEvent::Input(input));
                },
            },
            Ok(None) => {}
            Err(e) => {
                print!("{:?}\r\n", e);
                break;
            }
        }
    }

    Ok(())
}
