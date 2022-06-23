//! User shell

use alloc::string::String;
use futures_util::StreamExt;

use crate::{
    keyboard::ScancodeStream,
    print, println,
    ps2_keyboard_decoder::{ColemakDHm, DecodedKey, HandleControl, Keyboard, ScancodeSet1},
};

/// Represents a user shell.
///
/// Current capabilities are fairly limited. It just echos back what the user types.
pub struct Shell {
    scancodes: ScancodeStream,
    keyboard: Keyboard<ColemakDHm, ScancodeSet1>,
}

impl Shell {
    /// Create a new shell
    pub fn new(scancodes: ScancodeStream, keyboard: Keyboard<ColemakDHm, ScancodeSet1>) -> Self {
        Shell {
            scancodes,
            keyboard,
        }
    }

    /// Print out the default shell prompt.
    async fn print_prompt(&self) {
        print!("rosy> ");
    }

    /// Get input from the user. It reads the value from the [`ScancodeStream`].
    async fn get_input_while_echoing(&mut self) -> String {
        let mut command = String::new();

        while let Some(scancode) = self.scancodes.next().await {
            if let Ok(Some(key_event)) = self.keyboard.add_byte(scancode) {
                if let Some(key) = self.keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(character) => {
                            print!("{}", character);
                            if character == '\n' {
                                break;
                            }
                            command.push(character);
                        }
                        DecodedKey::RawKey(key) => print!("{:?}", key),
                    }
                }
            }
        }

        command
    }

    /// Run a shell loop
    pub async fn run(&mut self) {
        loop {
            self.print_prompt().await;
            let command = self.get_input_while_echoing().await;
            println!("{}", command);
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Shell::new(
            ScancodeStream::new(),
            Keyboard::new(ColemakDHm, ScancodeSet1, HandleControl::Ignore),
        )
    }
}
