//! Data model for argot commands.
//!
//! Every item in the argot command tree is represented by a [`Command`]. Related
//! types — [`Argument`], [`Flag`], [`Example`] — attach metadata that drives
//! both parsing and documentation generation.
//!
//! ## Builder Pattern
//!
//! All model types are constructed through consuming builders:
//!
//! ```
//! # use argot::model::{Command, Argument, Flag, Example};
//! let cmd = Command::builder("deploy")
//!     .summary("Deploy the application")
//!     .argument(
//!         Argument::builder("env")
//!             .description("Target environment")
//!             .required()
//!             .build()
//!             .unwrap(),
//!     )
//!     .flag(
//!         Flag::builder("dry-run")
//!             .short('n')
//!             .description("Simulate without making changes")
//!             .build()
//!             .unwrap(),
//!     )
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(cmd.canonical, "deploy");
//! ```
//!
//! ## Handler Functions and Parsed Commands
//!
//! A [`HandlerFn`] is an `Arc`-wrapped closure that receives a [`ParsedCommand`]
//! reference and returns `Result<(), Box<dyn Error>>`. The `Arc` wrapper means
//! cloning a [`Command`] only bumps a reference count — no deep copy of the
//! closure occurs.
//!
//! [`ParsedCommand`] is the output of a successful parse: it borrows the matched
//! [`Command`] from the registry and owns the resolved argument and flag maps.

/// Positional argument definition and builder.
pub mod argument;
/// Command definition, builder, handler type, and parsed command output.
pub mod command;
/// Usage example type for commands.
pub mod example;
/// Named flag definition and builder.
pub mod flag;

pub use argument::{Argument, ArgumentBuilder};
pub use command::{Command, CommandBuilder, HandlerFn, ParsedCommand};
pub use example::Example;
pub use flag::{Flag, FlagBuilder};

use thiserror::Error;

/// Error returned by builder `build()` methods.
///
/// Currently the only variant is [`BuildError::EmptyCanonical`], which is
/// returned when a [`CommandBuilder`] or [`ArgumentBuilder`] or [`FlagBuilder`]
/// is built with an empty or whitespace-only name.
///
/// # Examples
///
/// ```
/// # use argot::model::{Command, BuildError};
/// assert_eq!(Command::builder("").build().unwrap_err(), BuildError::EmptyCanonical);
/// ```
#[derive(Debug, Error, PartialEq)]
pub enum BuildError {
    /// The canonical name (or argument/flag name) was empty or whitespace.
    #[error("canonical name must not be empty")]
    EmptyCanonical,
}
