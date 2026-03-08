#[cfg(feature = "derive")]
mod tests {
    use argot::ArgotCommand;

    #[derive(ArgotCommand)]
    #[argot(
        summary = "Deploy the application",
        alias = "d",
        best_practice = "always dry-run first"
    )]
    struct Deploy {
        #[argot(positional, required, description = "target environment")]
        env: String,

        #[argot(flag, short = 'n', description = "dry run mode")]
        dry_run: bool,

        #[argot(
            flag,
            short = 'o',
            takes_value,
            description = "output format",
            default = "text"
        )]
        output: String,
    }

    #[test]
    fn test_canonical_name_from_struct() {
        let cmd = Deploy::command();
        assert_eq!(cmd.canonical, "deploy");
    }

    #[test]
    fn test_summary_and_alias() {
        let cmd = Deploy::command();
        assert_eq!(cmd.summary, "Deploy the application");
        assert!(cmd.aliases.contains(&"d".to_string()));
    }

    #[test]
    fn test_positional_argument() {
        let cmd = Deploy::command();
        let arg = cmd.arguments.iter().find(|a| a.name == "env").unwrap();
        assert!(arg.required);
        assert_eq!(arg.description, "target environment");
    }

    #[test]
    fn test_flag_boolean() {
        let cmd = Deploy::command();
        let flag = cmd.flags.iter().find(|f| f.name == "dry-run").unwrap();
        assert_eq!(flag.short, Some('n'));
        assert!(!flag.takes_value);
    }

    #[test]
    fn test_flag_with_value_and_default() {
        let cmd = Deploy::command();
        let flag = cmd.flags.iter().find(|f| f.name == "output").unwrap();
        assert!(flag.takes_value);
        assert_eq!(flag.default.as_deref(), Some("text"));
    }

    #[test]
    fn test_best_practice() {
        let cmd = Deploy::command();
        assert!(cmd
            .best_practices
            .contains(&"always dry-run first".to_string()));
    }

    #[derive(ArgotCommand)]
    #[argot(canonical = "explicit-name")]
    struct SomeOtherCommand {}

    #[test]
    fn test_canonical_override() {
        let cmd = SomeOtherCommand::command();
        assert_eq!(cmd.canonical, "explicit-name");
    }
}
