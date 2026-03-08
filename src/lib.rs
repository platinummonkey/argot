pub mod model;
pub mod parser;
pub mod query;
pub mod render;
pub mod resolver;

pub use model::{
    Argument, ArgumentBuilder, BuildError, Command, CommandBuilder, Example, Flag, FlagBuilder,
    HandlerFn, ParsedCommand,
};
pub use parser::{ParseError, Parser};
pub use query::{QueryError, Registry};
pub use render::{render_ambiguity, render_help, render_markdown, render_subcommand_list};
pub use resolver::{ResolveError, Resolver};

/// Trait implemented by types annotated with `#[derive(ArgotCommand)]`.
///
/// Call `T::command()` to obtain a fully-built [`Command`] from the struct's
/// `#[argot(...)]` attributes.
pub trait ArgotCommand {
    fn command() -> Command;
}

#[cfg(feature = "derive")]
pub use argot_derive::ArgotCommand;
