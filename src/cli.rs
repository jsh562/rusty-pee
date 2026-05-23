//! Command-line interface — clap derive `Cli` struct.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "rusty-pee",
    version,
    about = "Fan stdin out to N concurrent shell-spawned children.",
    long_about = "A Rust port of moreutils `pee`. Reads stdin once and writes \
                  every byte to each child's stdin in argv order; aggregates \
                  exit codes; surfaces failures cleanly."
)]
pub struct Cli {
    /// Buffer each child's stdout and emit in argv order after all children
    /// exit (Default mode only). Without this flag, children inherit the
    /// parent's stdout and their outputs interleave nondeterministically.
    #[arg(long)]
    pub capture: bool,

    /// Enable strict moreutils-compat mode.
    #[arg(long, conflicts_with = "no_strict")]
    pub strict: bool,

    /// Explicitly disable strict mode (overrides env + argv[0]).
    #[arg(long = "no-strict")]
    pub no_strict: bool,

    /// Positional command strings — each spawned via the platform shell.
    #[arg(trailing_var_arg = true)]
    pub commands: Vec<String>,

    /// Subcommand (currently only `completions`).
    #[command(subcommand)]
    pub command: Option<Subcommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Emit shell completion scripts (Default mode only).
    Completions { shell: clap_complete::Shell },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_command_factory_compiles() {
        let cmd = Cli::command();
        assert_eq!(cmd.get_name(), "rusty-pee");
    }

    #[test]
    fn parse_no_args() {
        let cli = Cli::try_parse_from(["rusty-pee"]).unwrap();
        assert!(cli.commands.is_empty());
        assert!(!cli.capture);
        assert!(!cli.strict);
        assert!(!cli.no_strict);
    }

    #[test]
    fn parse_capture_flag() {
        let cli = Cli::try_parse_from(["rusty-pee", "--capture", "cat"]).unwrap();
        assert!(cli.capture);
        assert_eq!(cli.commands, vec!["cat"]);
    }

    #[test]
    fn parse_strict_conflicts_with_no_strict() {
        let result = Cli::try_parse_from(["rusty-pee", "--strict", "--no-strict"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_two_commands() {
        let cli = Cli::try_parse_from(["rusty-pee", "wc -l", "grep foo"]).unwrap();
        assert_eq!(cli.commands, vec!["wc -l", "grep foo"]);
    }
}
