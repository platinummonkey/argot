pub mod argument;
pub mod command;
pub mod example;
pub mod flag;

pub use argument::{Argument, ArgumentBuilder};
pub use command::{Command, CommandBuilder, HandlerFn, ParsedCommand};
pub use example::Example;
pub use flag::{Flag, FlagBuilder};

use thiserror::Error;

/// Error returned by builder `build()` methods.
#[derive(Debug, Error, PartialEq)]
pub enum BuildError {
    #[error("canonical name must not be empty")]
    EmptyCanonical,
}
