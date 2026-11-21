//! Shell completion scripts (spec §6's [CP] `completions` verb):
//! hand-written static scripts over the pinned verb and flag surface.
//! They list verbs, global flags, and per-verb flag hints; a parser
//! crate would own generated completions, and there is none.

/// The completion script for `bash`, `zsh`, or `fish`.
pub fn script(shell: &str) -> String {
    match shell {
        "bash" => BASH.to_owned(),
        "zsh" => ZSH.to_owned(),
        "fish" => FISH.to_owned(),
        _ => unreachable!("the verb validates the shell"),
    }
}

const BASH: &str = r#"_commonplace_cli_completions()
{
    local cur="${COMP_WORDS[COMP_CWORD]}"
    if [ "$COMP_CWORD" -eq 1 ]; then
        COMPREPLY=( $(compgen -W "search list open new append export tags cat render config completions scheme" -- "$cur") )
        return
    fi
    case "${COMP_WORDS[1]}" in
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") ) ;;
        scheme)
            COMPREPLY=( $(compgen -W "find make markdownify" -- "$cur") ) ;;
        *)
            COMPREPLY=( $(compgen -W "--json --dir --sort --reverse --out --format --body --text --stdin --saved --get --set --unset --title --txt --tags --html --url" -- "$cur") ) ;;
    esac
}
complete -F _commonplace_cli_completions commonplace-cli
"#;

const ZSH: &str = r#"#compdef commonplace-cli
_commonplace_cli() {
    local -a verbs
    verbs=(search list open new append export tags cat render config completions scheme)
    if (( CURRENT == 2 )); then
        _describe 'verb' verbs
        return
    fi
    case "$words[2]" in
        completions)
            _values 'shell' bash zsh fish ;;
        scheme)
            _values 'action' find make markdownify ;;
        *)
            _commonplace_cli_flags ;;
    esac
}
_commonplace_cli_flags() {
    local -a flags
    flags=(--json --dir --sort --reverse --out --format --body --text --stdin --saved --get --set --unset --title --txt --tags --html --url)
    _describe 'flag' flags
}
complete -F _commonplace_cli commonplace-cli
"#;

const FISH: &str = r#"complete -c commonplace-cli -n "__fish_use_subcommand" -a "search list open new append export tags cat render config completions scheme"
complete -c commonplace-cli -n "__fish_seen_subcommand_from completions" -a "bash zsh fish"
complete -c commonplace-cli -n "__fish_seen_subcommand_from scheme" -a "find make markdownify"
complete -c commonplace-cli -l json -d "print the JSON envelope"
complete -c commonplace-cli -l dir -r -d "override the notes folder"
complete -c commonplace-cli -l sort -r -a "title modified created" -d "sort column"
complete -c commonplace-cli -l reverse -d "flip the sort direction"
complete -c commonplace-cli -l out -r -d "export output directory"
complete -c commonplace-cli -l format -r -a "text html" -d "export format"
complete -c commonplace-cli -l body -r -d "body text for new"
complete -c commonplace-cli -l text -r -d "text for append"
complete -c commonplace-cli -l stdin -d "read body/text from stdin"
complete -c commonplace-cli -l saved -r -a "0 1 2 3 4 5 6 7 8 9" -d "saved search slot"
complete -c commonplace-cli -l get -r -d "config: read a key"
complete -c commonplace-cli -l set -r -d "config: write a key"
complete -c commonplace-cli -l unset -r -d "config: remove a key"
complete -c commonplace-cli -l title -r -d "scheme make: the note title"
complete -c commonplace-cli -l txt -r -d "scheme make: the note body"
complete -c commonplace-cli -l tags -r -d "scheme make: tags (Phase 21)"
complete -c commonplace-cli -l html -r -d "scheme make: HTML body (Phase 21)"
complete -c commonplace-cli -l url -r -d "scheme make: source URL (Phase 21)"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    const VERBS: &str =
        "search list open new append export tags cat render config completions scheme";

    #[test]
    fn scripts_name_the_whole_verb_surface() {
        for shell in ["bash", "zsh", "fish"] {
            let s = script(shell);
            for verb in VERBS.split_whitespace() {
                assert!(s.contains(verb), "{shell} completion names `{verb}`");
            }
        }
    }
}
