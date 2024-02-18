use serde_json::Value;

use substrate_constructor::fill_prepare::TransactionToFill;

use termwiz::caps::Capabilities;
use termwiz::cell::AttributeChange;
use termwiz::color::AnsiColor;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, CursorVisibility, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{new_terminal, Terminal};
use termwiz::Error;

use tokio::sync::mpsc;

mod chain;

mod author;
use author::AddressBook;

mod call;
use call::CallField;

mod details;
use details::Details;

mod extrinsic_builder;
use extrinsic_builder::Builder;

mod scaffold;
use scaffold::Scaffold;

#[tokio::main]
async fn main() -> Result<(), Error> {

    let (mut block_hash_rx, mut block_rx) = chain::block_watch();

    let some_block = block_rx.recv().await.unwrap();
    let metadata = chain::get_metadata(some_block).await;
    let genesis_hash = chain::get_genesis_hash().await;
    let specs = chain::get_specs(some_block).await;
    let ss58 = if let Some(Value::Number(a)) = specs.get("ss58Format") {
            if let Some(b) = a.as_u64() {
                b as u16
            } else {42}
        } else {42};

    let address_book = AddressBook::init(ss58);

    let mut builder = Builder::new(&metadata, &address_book, genesis_hash, specs);
    let mut hash = some_block;

    let caps = Capabilities::new_from_env()?;

    let terminal = new_terminal(caps)?;

    let mut buf = BufferedTerminal::new(terminal)?;
    buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));

    buf.terminal().set_raw_mode()?;

    let mut scaffold = Scaffold::new(buf.terminal().get_screen_size()?);

    let mut header = scaffold.header().surface();
    header.add_change(Change::ClearScreen(AnsiColor::Blue.into()));
    header.add_change("=====Substrate low-fi client!=====");
    buf.draw_from_screen(
        &header,
        scaffold.header().column(),
        scaffold.header().line(),
    );
    let mut separator = scaffold.details_separator().surface();
    separator.add_change(Change::ClearScreen(AnsiColor::Blue.into()));
    buf.draw_from_screen(
        &separator,
        scaffold.details_separator().column(),
        scaffold.details_separator().line(),
    );
    buf.flush()?;

    let mut block = scaffold.block().surface();

    let mut call_field = CallField::new(scaffold.call().surface());
    buf.draw_from_screen(
        call_field.render(builder.call(), &builder.position()),
        scaffold.call().column(),
        scaffold.call().line(),
    );

    let mut details_field = Details::new(scaffold.details_panel().surface());
    buf.draw_from_screen(
        details_field.render(builder.details()),
        scaffold.details_panel().column(),
        scaffold.details_panel().line(),
    );

    loop {
        if builder.details {
            buf.add_change(Change::CursorVisibility(CursorVisibility::Visible));
        } else {
            buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
        }
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
                }
                InputEvent::Paste(s) => builder.paste(s),
                // Area-specific buttons
                InputEvent::Key(key) => {
                    match key {
                        KeyEvent {
                            key: KeyCode::UpArrow,
                            ..
                        } => {
                            builder.up();
                        }
                        KeyEvent {
                            key: KeyCode::DownArrow,
                            ..
                        } => {
                            builder.down();
                        }
                        KeyEvent {
                            key: KeyCode::LeftArrow,
                            ..
                        } => {
                            builder.left();
                        }
                        KeyEvent {
                            key: KeyCode::RightArrow,
                            ..
                        } => {
                            builder.right();
                        }
                        KeyEvent {
                            key: KeyCode::Enter,
                            ..
                        } => {
                            builder.enter();
                        }
                        KeyEvent {
                            key: KeyCode::Backspace,
                            ..
                        } => {
                            builder.backspace();
                        }
                        KeyEvent {
                            key: KeyCode::Char(c),
                            ..
                        } => {
                            builder.input(c);
                        }

                        _ => {}
                    };
                    builder.autofill(hash);
                    buf.draw_from_screen(
                        call_field.render(builder.call(), &builder.position()),
                        scaffold.call().column(),
                        scaffold.call().line(),
                    );
                    buf.draw_from_screen(
                        details_field.render(builder.details()),
                        scaffold.details_panel().column(),
                        scaffold.details_panel().line(),
                    );
                }
                _ => {}
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
