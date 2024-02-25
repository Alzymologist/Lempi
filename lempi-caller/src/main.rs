use clap::Parser;

use std::time::Duration;

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

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = String::from("polkadot"))]
    chainspec: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    let mut bc = chain::Blockchain::new(&format!("../chain-specs/{}.json", args.chainspec)).await;

    // TODO: this should be a reference and builder should die and be reborn
    // if this changes
    let metadata = bc.metadata().clone();
    let genesis_hash = bc.genesis_hash();
    let specs = bc.specs();
    let ss58 = if let Some(Value::Number(a)) = specs.get("ss58Format") {
        if let Some(b) = a.as_u64() {
            b as u16
        } else {
            42
        }
    } else {
        42
    };

    let address_book = AddressBook::init(ss58);

    let mut builder = Builder::new(&metadata, &address_book, genesis_hash, specs);
    let mut hash = bc.block();

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

    let mut log_field = scaffold.logger().surface();

    loop {
        log_field.add_change(builder.log());
        log_field.add_change(bc.log());
        buf.draw_from_screen(
            &log_field,
            scaffold.logger().column(),
            scaffold.logger().line(),
        );

        //        if builder.details {
        //            buf.add_change(Change::CursorVisibility(CursorVisibility::Visible));
        //        } else {
        buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
        //        }

        let updated = bc.crank();
        let nonce = if let Some(a) = builder.author() {
            bc.nonce(a, builder.ss58)
        } else {
            None
        };
        if updated {
            builder.autofill(hash, nonce);
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
            hash = bc.block();
            block.add_change(Change::ClearScreen(AnsiColor::Grey.into()));
            block.add_change(format!("Last block: {}", &hash));
            buf.draw_from_screen(&block, scaffold.block().column(), scaffold.block().line());
        }

        buf.flush()?;

        match buf.terminal().poll_input(Some(Duration::new(1, 0))) {
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
                            key: KeyCode::Tab, ..
                        } => {
                            if let Some(a) = builder.submittable_signed() {
                                bc.send(&a);
                            }
                        }
                        KeyEvent {
                            key: KeyCode::Char(c),
                            ..
                        } => {
                            builder.input(c);
                        }

                        _ => {}
                    };
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
