//! Hand-rolled argument parsing (spec §6: stdlib only; a parser crate
//! would be a deliberate decision). Unknown verbs, unknown flags, flags
//! that do not apply to the verb, and wrong positional counts are
//! usage errors (exit 2).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Search,
    List,
    Open,
    New,
    Append,
    Export,
    Tags,
    Cat,
    Render,
    Config,
    Completions,
    Scheme,
}

impl Verb {
    pub fn name(self) -> &'static str {
        match self {
            Verb::Search => "search",
            Verb::List => "list",
            Verb::Open => "open",
            Verb::New => "new",
            Verb::Append => "append",
            Verb::Export => "export",
            Verb::Tags => "tags",
            Verb::Cat => "cat",
            Verb::Render => "render",
            Verb::Config => "config",
            Verb::Completions => "completions",
            Verb::Scheme => "scheme",
        }
    }

    fn from_name(name: &str) -> Option<Verb> {
        Some(match name {
            "search" => Verb::Search,
            "list" => Verb::List,
            "open" => Verb::Open,
            "new" => Verb::New,
            "append" => Verb::Append,
            "export" => Verb::Export,
            "tags" => Verb::Tags,
            "cat" => Verb::Cat,
            "render" => Verb::Render,
            "config" => Verb::Config,
            "completions" => Verb::Completions,
            "scheme" => Verb::Scheme,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortArg {
    Title,
    Modified,
    Created,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Flags {
    pub json: bool,
    pub dir: Option<String>,
    pub sort: Option<SortArg>,
    pub reverse: bool,
    pub out: Option<String>,
    pub format: Option<String>,
    pub body: Option<String>,
    pub text: Option<String>,
    pub stdin: bool,
    pub saved: Option<u8>,
    pub get: Option<String>,
    pub set: Option<(String, String)>,
    pub unset: Option<String>,
    pub title: Option<String>,
    pub txt: Option<String>,
    pub tags: Option<String>,
    pub html: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    pub verb: Verb,
    pub flags: Flags,
    pub positionals: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnknownVerb(String),
    UnknownFlag(String),
    FlagNotForVerb {
        flag: &'static str,
        verb: &'static str,
    },
    MissingFlagValue(String),
    BadFlagValue {
        flag: String,
        reason: String,
    },
    BadPositionalCount {
        verb: &'static str,
        reason: String,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownVerb(v) => write!(f, "unknown verb `{v}`"),
            ParseError::UnknownFlag(flag) => write!(f, "unknown flag `{flag}`"),
            ParseError::FlagNotForVerb { flag, verb } => {
                write!(f, "flag `{flag}` does not apply to `{verb}`")
            }
            ParseError::MissingFlagValue(flag) => write!(f, "flag `{flag}` needs a value"),
            ParseError::BadFlagValue { flag, reason } => {
                write!(f, "flag `{flag}`: {reason}")
            }
            ParseError::BadPositionalCount { verb, reason } => {
                write!(f, "`{verb}`: {reason}")
            }
        }
    }
}

/// Parse `argv` (without the program name) into an invocation.
pub fn parse(argv: &[String]) -> Result<Invocation, ParseError> {
    let Some(verb_name) = argv.first() else {
        return Err(ParseError::UnknownVerb(String::new()));
    };
    let Some(verb) = Verb::from_name(verb_name) else {
        return Err(ParseError::UnknownVerb(verb_name.clone()));
    };

    let mut flags = Flags::default();
    let mut positionals: Vec<String> = Vec::new();
    let mut rest = &argv[1..];
    while let Some(arg) = rest.first() {
        rest = &rest[1..];
        if let Some(flag) = arg.strip_prefix("--") {
            match flag {
                "json" => flags.json = true,
                "reverse" => flags.reverse = true,
                "stdin" => flags.stdin = true,
                "dir" => flags.dir = Some(next_value(flag, &mut rest)?),
                "sort" => {
                    let value = next_value(flag, &mut rest)?;
                    flags.sort = Some(match value.as_str() {
                        "title" => SortArg::Title,
                        "modified" => SortArg::Modified,
                        "created" => SortArg::Created,
                        _ => {
                            return Err(ParseError::BadFlagValue {
                                flag: "--sort".to_owned(),
                                reason: "must be title, modified, or created".to_owned(),
                            });
                        }
                    });
                }
                "out" => flags.out = Some(next_value(flag, &mut rest)?),
                "format" => flags.format = Some(next_value(flag, &mut rest)?),
                "body" => flags.body = Some(next_value(flag, &mut rest)?),
                "text" => flags.text = Some(next_value(flag, &mut rest)?),
                "saved" => {
                    let value = next_value(flag, &mut rest)?;
                    let slot: u8 = value.parse().map_err(|_| ParseError::BadFlagValue {
                        flag: "--saved".to_owned(),
                        reason: "must be 0-9".to_owned(),
                    })?;
                    if slot > 9 {
                        return Err(ParseError::BadFlagValue {
                            flag: "--saved".to_owned(),
                            reason: "must be 0-9".to_owned(),
                        });
                    }
                    flags.saved = Some(slot);
                }
                "get" => flags.get = Some(next_value(flag, &mut rest)?),
                "set" => {
                    let key = next_value(flag, &mut rest)?;
                    let value = next_value(flag, &mut rest)?;
                    flags.set = Some((key, value));
                }
                "unset" => flags.unset = Some(next_value(flag, &mut rest)?),
                "title" => flags.title = Some(next_value(flag, &mut rest)?),
                "txt" => flags.txt = Some(next_value(flag, &mut rest)?),
                "tags" => flags.tags = Some(next_value(flag, &mut rest)?),
                "html" => flags.html = Some(next_value(flag, &mut rest)?),
                "url" => flags.url = Some(next_value(flag, &mut rest)?),
                other => return Err(ParseError::UnknownFlag(format!("--{other}"))),
            }
        } else {
            positionals.push(arg.clone());
        }
    }

    check_positionals(verb, &positionals)?;
    check_flag_ownership(verb, &flags)?;
    Ok(Invocation {
        verb,
        flags,
        positionals,
    })
}

fn next_value(flag: &str, rest: &mut &[String]) -> Result<String, ParseError> {
    let Some(value) = rest.first() else {
        return Err(ParseError::MissingFlagValue(flag.to_owned()));
    };
    *rest = &rest[1..];
    Ok(value.clone())
}

fn check_positionals(verb: Verb, positionals: &[String]) -> Result<(), ParseError> {
    let (min, max) = match verb {
        Verb::Search | Verb::Export => (0, usize::MAX),
        Verb::List | Verb::Tags | Verb::Config => (0, 0),
        Verb::Open | Verb::Cat | Verb::Render | Verb::Append => (1, 1),
        Verb::New => (1, usize::MAX),
        Verb::Completions => (1, 1),
        Verb::Scheme => (1, usize::MAX),
    };
    if positionals.len() < min {
        return Err(ParseError::BadPositionalCount {
            verb: verb.name(),
            reason: match min {
                1 => "needs one argument".to_owned(),
                _ => "needs arguments".to_owned(),
            },
        });
    }
    if positionals.len() > max {
        return Err(ParseError::BadPositionalCount {
            verb: verb.name(),
            reason: "takes no arguments".to_owned(),
        });
    }
    Ok(())
}

/// Which flags each verb accepts; a flag that belongs to another verb
/// is a usage error rather than silent ignoring.
fn check_flag_ownership(verb: Verb, flags: &Flags) -> Result<(), ParseError> {
    let bad = |flag: &'static str| {
        Err(ParseError::FlagNotForVerb {
            flag,
            verb: verb.name(),
        })
    };
    match verb {
        Verb::Search => {
            if flags.out.is_some() {
                return bad("--out");
            }
            if flags.format.is_some() {
                return bad("--format");
            }
        }
        Verb::List => {
            if flags.saved.is_some() {
                return bad("--saved");
            }
        }
        Verb::Open | Verb::Cat => {
            if flags.sort.is_some() || flags.reverse {
                return bad("--sort");
            }
            if flags.out.is_some() {
                return bad("--out");
            }
        }
        Verb::New => {
            if flags.sort.is_some() || flags.reverse {
                return bad("--sort");
            }
        }
        Verb::Append => {
            if flags.sort.is_some() || flags.reverse {
                return bad("--sort");
            }
        }
        Verb::Export => {
            if flags.saved.is_some() {
                return bad("--saved");
            }
        }
        Verb::Tags | Verb::Render => {
            if flags.sort.is_some() || flags.reverse {
                return bad("--sort");
            }
            if flags.out.is_some() {
                return bad("--out");
            }
        }
        Verb::Config => {
            let set = flags.set.is_some() as u8 + flags.unset.is_some() as u8;
            if set > 1 {
                return Err(ParseError::BadFlagValue {
                    flag: "--set/--unset".to_owned(),
                    reason: "only one of --set/--unset per call".to_owned(),
                });
            }
        }
        Verb::Completions => {
            if flags.dir.is_some() {
                return bad("--dir");
            }
        }
        Verb::Scheme => {
            if flags.sort.is_some() {
                return bad("--sort");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_flags_and_positionals() {
        let inv = parse(&argv(&["search", "--json", "alpha", "beta"])).unwrap();
        assert_eq!(inv.verb, Verb::Search);
        assert!(inv.flags.json);
        assert_eq!(inv.positionals, vec!["alpha", "beta"]);
    }

    #[test]
    fn unknown_verb_and_flag_are_parse_errors() {
        assert_eq!(
            parse(&argv(&["frobnicate"])),
            Err(ParseError::UnknownVerb("frobnicate".to_owned()))
        );
        assert_eq!(
            parse(&argv(&["list", "--wat"])),
            Err(ParseError::UnknownFlag("--wat".to_owned()))
        );
        assert_eq!(
            parse(&argv(&[])),
            Err(ParseError::UnknownVerb(String::new()))
        );
    }

    #[test]
    fn positional_counts_are_enforced() {
        assert!(parse(&argv(&["open"])).is_err());
        assert!(parse(&argv(&["open", "a", "b"])).is_err());
        assert!(parse(&argv(&["new"])).is_err());
        assert!(parse(&argv(&["tags", "extra"])).is_err());
        assert!(parse(&argv(&["completions"])).is_err());
        assert!(parse(&argv(&["new", "one", "two"])).is_ok());
    }

    #[test]
    fn flag_ownership_is_enforced() {
        assert!(parse(&argv(&["list", "--saved", "3"])).is_err());
        assert!(parse(&argv(&["search", "--out", "x"])).is_err());
        assert!(parse(&argv(&["open", "--sort", "title"])).is_err());
        assert!(parse(&argv(&["config", "--set", "a", "b", "--unset", "a"])).is_err());
    }

    #[test]
    fn saved_slots_bound_to_0_9() {
        assert!(parse(&argv(&["search", "--saved", "9"])).is_ok());
        assert!(parse(&argv(&["search", "--saved", "0"])).is_ok());
        assert!(parse(&argv(&["search", "--saved", "10"])).is_err());
        assert!(parse(&argv(&["search", "--saved", "x"])).is_err());
    }
}
