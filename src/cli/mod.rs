//! High-level CLI entry point that wires together the argot pipeline.
//!
//! [`Cli`] is a batteries-included struct that combines [`Registry`],
//! [`Parser`] and the render layer into a single `run` method.
//! It handles the common built-in behaviors (help, version, empty input) so
//! that application code only needs to build commands and register handlers.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use argot::{Cli, Command};
//!
//! let cmd = Command::builder("greet")
//!     .summary("Say hello")
//!     .handler(Arc::new(|_| {
//!         println!("Hello, world!");
//!         Ok(())
//!     }))
//!     .build()
//!     .unwrap();
//!
//! let cli = Cli::new(vec![cmd])
//!     .app_name("myapp")
//!     .version("1.0.0");
//!
//! // In a real application:
//! // cli.run_env_args().unwrap();
//! ```

use crate::parser::{ParseError, Parser};
use crate::query::Registry;
use crate::render::{render_help, render_subcommand_list};
use crate::resolver::Resolver;

/// Errors produced by [`Cli::run`].
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A parse error occurred (unknown command, missing argument, etc.).
    ///
    /// When this variant is returned, `Cli::run` also prints the error and
    /// best-effort help to stderr before returning.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The matched command has no handler registered.
    ///
    /// The inner `String` is the canonical name of the command.
    #[error("command `{0}` has no handler registered")]
    NoHandler(String),
    /// The registered handler returned an error.
    ///
    /// The inner boxed error carries the handler's error message.
    #[error("handler error: {0}")]
    Handler(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A batteries-included entry point that wires together [`Registry`], [`Parser`],
/// and the render layer so callers do not have to do it themselves.
///
/// Build a `Cli` with [`Cli::new`], optionally configure it with
/// [`Cli::app_name`] and [`Cli::version`], then call [`Cli::run`] (or
/// [`Cli::run_env_args`] for the common case of reading from
/// [`std::env::args`]).
///
/// ## Built-in behaviors
///
/// | Input | Behavior |
/// |-------|----------|
/// | `--help` / `-h` anywhere | Print help for the most-specific resolved command; return `Ok(())`. |
/// | `--version` / `-V` | Print `"<app_name> <version>"` (or just the version); return `Ok(())`. |
/// | Empty argument list | Print the top-level command listing; return `Ok(())`. |
/// | Unrecognized command | Print error + help to stderr; return `Err(CliError::Parse(...))`. |
///
/// # Examples
///
/// ```
/// # use std::sync::Arc;
/// # use argot::{Cli, Command};
/// let cli = Cli::new(vec![
///     Command::builder("ping")
///         .summary("Check connectivity")
///         .handler(Arc::new(|_| { println!("pong"); Ok(()) }))
///         .build()
///         .unwrap(),
/// ])
/// .app_name("myapp")
/// .version("0.1.0");
///
/// // Invoking with no args prints the command list (does not error).
/// assert!(cli.run(std::iter::empty::<&str>()).is_ok());
/// ```
pub struct Cli {
    registry: Registry,
    app_name: String,
    version: Option<String>,
}

impl Cli {
    /// Create a new `Cli` from a list of top-level commands.
    ///
    /// # Arguments
    ///
    /// - `commands` — The fully-built top-level command list. Ownership is
    ///   transferred to an internal [`Registry`].
    pub fn new(commands: Vec<crate::model::Command>) -> Self {
        Self {
            registry: Registry::new(commands),
            app_name: String::new(),
            version: None,
        }
    }

    /// Set the application name (shown in version output and top-level help).
    ///
    /// If not set, the version string is printed without a prefix.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Set the application version (shown by `--version` / `-V`).
    ///
    /// If not set, `"(no version set)"` is printed.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Parse and dispatch a command from an iterator of string arguments.
    ///
    /// The iterator should **not** include the program name (`argv[0]`).
    ///
    /// Built-in behaviors:
    /// - `--help` or `-h` anywhere → print help for the most-specific matched
    ///   command and return `Ok(())`.
    /// - `--version` or `-V` → print version string and return `Ok(())`.
    /// - Empty input → print top-level command list and return `Ok(())`.
    /// - Parse error → print the error to stderr, then help if possible; return
    ///   `Err(CliError::Parse(...))`.
    /// - No handler registered → return `Err(CliError::NoHandler(...))`.
    ///
    /// # Arguments
    ///
    /// - `args` — Iterator of argument strings, not including the program name.
    ///
    /// # Errors
    ///
    /// - [`CliError::Parse`] — the argument list could not be parsed.
    /// - [`CliError::NoHandler`] — the resolved command has no handler.
    /// - [`CliError::Handler`] — the handler returned an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use argot::{Cli, Command, CliError};
    /// let cli = Cli::new(vec![
    ///     Command::builder("hello")
    ///         .handler(Arc::new(|_| Ok(())))
    ///         .build()
    ///         .unwrap(),
    /// ]);
    ///
    /// assert!(cli.run(["hello"]).is_ok());
    /// assert!(matches!(cli.run(["--help"]), Ok(())));
    /// ```
    pub fn run(&self, args: impl IntoIterator<Item = impl AsRef<str>>) -> Result<(), CliError> {
        let argv: Vec<String> = args.into_iter().map(|a| a.as_ref().to_owned()).collect();
        let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();

        // ── Built-in: --help / -h ──────────────────────────────────────────
        if argv_refs.iter().any(|a| *a == "--help" || *a == "-h") {
            // Strip the help flag(s) and try to identify the target command.
            let remaining: Vec<&str> = argv_refs
                .iter()
                .copied()
                .filter(|a| *a != "--help" && *a != "-h")
                .collect();

            let help_text = self.resolve_help_text(&remaining);
            print!("{}", help_text);
            return Ok(());
        }

        // ── Built-in: --version / -V ──────────────────────────────────────
        if argv_refs.iter().any(|a| *a == "--version" || *a == "-V") {
            match &self.version {
                Some(v) if !self.app_name.is_empty() => println!("{} {}", self.app_name, v),
                Some(v) => println!("{}", v),
                None => println!("(no version set)"),
            }
            return Ok(());
        }

        // ── Built-in: empty args → list top-level commands ────────────────
        if argv_refs.is_empty() {
            print!("{}", render_subcommand_list(self.registry.commands()));
            return Ok(());
        }

        // ── Normal parse ──────────────────────────────────────────────────
        let parser = Parser::new(self.registry.commands());
        match parser.parse(&argv_refs) {
            Ok(parsed) => {
                match &parsed.command.handler {
                    Some(handler) => {
                        // HandlerFn returns Box<dyn Error> (no Send+Sync bound).
                        // We convert manually to match CliError::Handler.
                        handler(&parsed).map_err(|e| {
                            // Wrap in a Send+Sync-compatible error by capturing
                            // the display string.
                            let msg = e.to_string();
                            let boxed: Box<dyn std::error::Error + Send + Sync> =
                                msg.into();
                            CliError::Handler(boxed)
                        })
                    }
                    None => Err(CliError::NoHandler(parsed.command.canonical.to_string())),
                }
            }
            Err(e) => {
                eprintln!("error: {}", e);
                // Best-effort: render help for whatever partial command we can resolve.
                let help_text = self.resolve_help_text(&argv_refs);
                eprint!("{}", help_text);
                Err(CliError::Parse(e))
            }
        }
    }

    /// Convenience: run with `std::env::args().skip(1)`.
    ///
    /// Equivalent to `self.run(std::env::args().skip(1))`. Skipping element 0
    /// is required because `std::env::args` includes the program name.
    ///
    /// # Errors
    ///
    /// Same as [`Cli::run`].
    pub fn run_env_args(&self) -> Result<(), CliError> {
        self.run(std::env::args().skip(1))
    }

    // ── Private helpers ───────────────────────────────────────────────────

    /// Walk the arg list and return the help text for the deepest command that
    /// can be resolved. Falls back to the top-level command list if nothing
    /// resolves.
    fn resolve_help_text(&self, argv: &[&str]) -> String {
        // Try to walk the command tree as far as possible.
        if argv.is_empty() {
            return render_subcommand_list(self.registry.commands());
        }

        // Skip any flag-looking tokens for the purpose of command resolution.
        let words: Vec<&str> = argv
            .iter()
            .copied()
            .filter(|a| !a.starts_with('-'))
            .collect();

        if words.is_empty() {
            return render_subcommand_list(self.registry.commands());
        }

        // Resolve the first word as a top-level command.
        let resolver = Resolver::new(self.registry.commands());
        let top_cmd = match resolver.resolve(words[0]) {
            Ok(cmd) => cmd,
            Err(_) => return render_subcommand_list(self.registry.commands()),
        };

        // Walk into subcommands as far as possible.
        let mut current = top_cmd;
        for word in words.iter().skip(1) {
            if current.subcommands.is_empty() {
                break;
            }
            let sub_resolver = Resolver::new(&current.subcommands);
            match sub_resolver.resolve(word) {
                Ok(sub) => current = sub,
                Err(_) => break,
            }
        }

        render_help(current)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Command;
    use std::sync::{Arc, Mutex};

    fn make_cli_no_handler() -> Cli {
        let cmd = Command::builder("greet")
            .summary("Say hello")
            .build()
            .unwrap();
        Cli::new(vec![cmd])
            .app_name("testapp")
            .version("1.2.3")
    }

    fn make_cli_with_handler(called: Arc<Mutex<bool>>) -> Cli {
        let cmd = Command::builder("greet")
            .summary("Say hello")
            .handler(Arc::new(move |_parsed| {
                *called.lock().unwrap() = true;
                Ok(())
            }))
            .build()
            .unwrap();
        Cli::new(vec![cmd])
            .app_name("testapp")
            .version("1.2.3")
    }

    #[test]
    fn test_run_empty_args() {
        let cli = make_cli_no_handler();
        let result = cli.run(std::iter::empty::<&str>());
        assert!(result.is_ok(), "empty args should return Ok");
    }

    #[test]
    fn test_run_help_flag() {
        let cli = make_cli_no_handler();
        let result = cli.run(["--help"]);
        assert!(result.is_ok(), "--help should return Ok");
    }

    #[test]
    fn test_run_help_flag_short() {
        let cli = make_cli_no_handler();
        let result = cli.run(["-h"]);
        assert!(result.is_ok(), "-h should return Ok");
    }

    #[test]
    fn test_run_version_flag() {
        let cli = make_cli_no_handler();
        let result = cli.run(["--version"]);
        assert!(result.is_ok(), "--version should return Ok");
    }

    #[test]
    fn test_run_version_flag_short() {
        let cli = make_cli_no_handler();
        let result = cli.run(["-V"]);
        assert!(result.is_ok(), "-V should return Ok");
    }

    #[test]
    fn test_run_no_handler() {
        let cli = make_cli_no_handler();
        let result = cli.run(["greet"]);
        assert!(
            matches!(result, Err(CliError::NoHandler(ref name)) if name == "greet"),
            "expected NoHandler(\"greet\"), got {:?}",
            result
        );
    }

    #[test]
    fn test_run_with_handler() {
        let called = Arc::new(Mutex::new(false));
        let cli = make_cli_with_handler(called.clone());
        let result = cli.run(["greet"]);
        assert!(result.is_ok(), "handler should succeed, got {:?}", result);
        assert!(*called.lock().unwrap(), "handler should have been called");
    }

    #[test]
    fn test_run_unknown_command() {
        let cli = make_cli_no_handler();
        let result = cli.run(["unknowncmd"]);
        assert!(
            matches!(result, Err(CliError::Parse(_))),
            "unknown command should yield Parse error, got {:?}",
            result
        );
    }
}
