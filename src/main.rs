use substrate_constructor::fill_prepare::TransactionToFill;


use termwiz::caps::Capabilities;
use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{new_terminal, Terminal};
use termwiz::Error;

use tokio::sync::mpsc;

mod chain;

mod author;
use author::AuthorField;

mod call;
use call::CallField;

mod extrinsic_builder;
use extrinsic_builder::{Builder, SelectedArea};

mod scaffold;
use scaffold::Scaffold;

#[tokio::main]
async fn main() -> Result<(), Error> {

    let (mut block_hash_rx, mut block_rx) = chain::block_watch();

    let some_block = block_rx.recv().await.unwrap();
    let metadata = chain::get_metadata(&some_block).await;

    let mut builder = Builder::new(metadata);
    let mut hash = String::new();

    let mut selected_area = SelectedArea::Call;

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

    buf.terminal().set_raw_mode()?;

    let mut scaffold = Scaffold::new(buf.terminal().get_screen_size()?);

    let mut header = scaffold.header().surface();
    header.add_change(Change::ClearScreen(AnsiColor::Blue.into()));
    header.add_change("=====Substrate low-fi client!=====");
    buf.draw_from_screen(&header, scaffold.header().column(), scaffold.header().line());
    buf.flush()?;

    let mut block = scaffold.block().surface();

    /*
    let mut extensions_field = Surface::new(screen_width, 10);
    extensions_field.add_change(format!("Extensions: {:?}", transaction.extensions));
    transaction_field.draw_from_screen(&extensions_field, 0, 24);
    */
   
    let mut author_field = AuthorField::new(scaffold.author().surface());
    buf.draw_from_screen(author_field.render(builder.author(), 0), scaffold.author().column(), scaffold.author().line());
    
    let mut call_field = CallField::new(scaffold.call().surface());
    buf.draw_from_screen(call_field.render(builder.call(), 0), scaffold.call().column(), scaffold.call().line());

    loop {
        if let Some(a) = chain::plop(&mut block_rx) {
            hash = a;
            block.add_change(Change::ClearScreen(AnsiColor::Grey.into()));
            block.add_change(format!("Last block: {}", &hash));
            buf.draw_from_screen(&block, scaffold.block().column(), scaffold.block().line());
        }

        buf.flush()?;

        match buf.terminal().poll_input(None) {
            Ok(Some(input)) => match input {
                // Important global buttons that do not care what is selected
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => {
                    buf.add_change(Change::ClearScreen(AnsiColor::Black.into()));
                    buf.flush()?;
                    break;
                },
                // Area-specific buttons
                InputEvent::Key(key) => {
                    match selected_area {
                        SelectedArea::Author => {
                            match key {
                                KeyEvent {
                                    key: KeyCode::Char(' '),
                                    ..
                                } => {},
                                _ => {},
                            };
                            buf.draw_from_screen(author_field.render(builder.author(), 0), scaffold.author().column(), scaffold.author().line());
                        },
                        SelectedArea::Call => {
                            match key {
                                KeyEvent {
                                    key: KeyCode::Char(' '),
                                    ..
                                } => {},
                                _ => {},
                            };
                            buf.draw_from_screen(call_field.render(builder.call(), 0), scaffold.call().column(), scaffold.call().line());
                        },
                        SelectedArea::Extensions => {},
                    }
                },
                _ => {},
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
