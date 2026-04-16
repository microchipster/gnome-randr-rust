use structopt::StructOpt;

const BIN_NAME: &str = env!("CARGO_PKG_NAME");
const BASH_FUNCTION_NAME: &str = "_gnome-randr()";
const ZSH_FUNCTION_NAME: &str = "_gnome-randr() {";

fn bash_command_root() -> String {
    BIN_NAME.replace('-', "__")
}

#[derive(StructOpt)]
pub struct CommandOptions {
    #[structopt(
        value_name = "SHELL",
        possible_values = &["bash", "zsh", "fish"],
        help = "Shell name bash, zsh, or fish",
        long_help = "Shell name to generate completions for. Valid values are \"bash\", \"zsh\", and \"fish\"."
    )]
    shell: String,
}

pub fn handle(opts: &CommandOptions) {
    use structopt::clap::Shell;

    let shell = match opts.shell.as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        _ => unreachable!(),
    };

    let mut app = super::build_cli();
    let mut buffer = Vec::new();
    app.gen_completions_to(BIN_NAME, shell, &mut buffer);

    let generated = String::from_utf8(buffer).expect("clap generated invalid UTF-8");
    let script = match shell {
        Shell::Bash => augment_bash(generated),
        Shell::Zsh => augment_zsh(generated),
        Shell::Fish => augment_fish(generated),
        _ => unreachable!(),
    };

    print!("{}", script);
}

fn augment_bash(generated: String) -> String {
    let mut script = generated.replacen(BASH_FUNCTION_NAME, "__gnome_randr_static()", 1);
    script = script.replacen(
        &format!("cmd=\"{}\"", BIN_NAME),
        &format!("cmd=\"{}\"", bash_command_root()),
        1,
    );
    script = script.replacen(
        &format!("case \"${{cmd}}\" in\n        {})", BIN_NAME),
        &format!("case \"${{cmd}}\" in\n        {})", bash_command_root()),
        1,
    );
    script.push_str(&format!(
        r#"
__gnome_randr_dynamic_values() {{
    local cur
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    command {bin} __complete "$cur" "${{COMP_WORDS[@]:1:COMP_CWORD-1}}" 2>/dev/null
}}

_gnome-randr() {{
    local cur
    local -a suggestions

    COMPREPLY=()
    cur="${{COMP_WORDS[COMP_CWORD]}}"

    mapfile -t suggestions < <(__gnome_randr_dynamic_values)
    if (( ${{#suggestions[@]}} > 0 )); then
        COMPREPLY=( $(compgen -W "${{suggestions[*]}}" -- "${{cur}}") )
        return 0
    fi

    __gnome_randr_static
}}
"#,
        bin = BIN_NAME
    ));
    script
}

fn augment_zsh(generated: String) -> String {
    let mut script = generated.replacen(ZSH_FUNCTION_NAME, "__gnome_randr_static() {", 1);
    script = script.replacen("_gnome-randr \"$@\"", "", 1);
    script.push_str(
        r#"
_gnome-randr() {
    local dynamic_output
    local -a suggestions prior_words

    prior_words=("${(@)words[2,CURRENT-1]}")
    dynamic_output=$(command gnome-randr __complete "$PREFIX" "${prior_words[@]}" 2>/dev/null)
    if [[ -n $dynamic_output ]]; then
        suggestions=(${(f)dynamic_output})
        compadd -- $suggestions
        return 0
    fi

    __gnome_randr_static "$@"
}

if [[ "${funcstack[1]-}" == "_gnome-randr" || "${funcstack[1]-}" == "_gnome_randr" ]]; then
    _gnome-randr "$@"
elif (( $+functions[compdef] )); then
    compdef _gnome-randr gnome-randr
fi
"#,
    );
    script
}

fn augment_fish(generated: String) -> String {
    format!(
        r#"{}

function __fish_gnome_randr_dynamic
    set -l tokens (commandline -opc)
    set -l current (commandline -ct)
    if test (count $tokens) -gt 0
        if test "$current" != ""; and test "$tokens[-1]" = "$current"
            set -e tokens[-1]
        end
        set -e tokens[1]
    end

    command {} "$current" $tokens 2>/dev/null
end

complete -c {} -f -a '(__fish_gnome_randr_dynamic)'
"#,
        generated, COMPLETE_FISH_COMMAND, BIN_NAME
    )
}

const COMPLETE_FISH_COMMAND: &str = "gnome-randr __complete";

#[cfg(test)]
mod tests {
    use super::{augment_bash, augment_zsh, BIN_NAME};

    #[test]
    fn bash_augmentation_normalizes_hyphenated_binary_name() {
        let script = augment_bash(format!(
            "_gnome-randr() {{\ncase \"${{i}}\" in\n        {})\n            cmd=\"{}\"\n            ;;\nesac\ncase \"${{cmd}}\" in\n        {})\n            ;;\n        {}__modify)\n            ;;\nesac\n}}\n",
            BIN_NAME,
            BIN_NAME,
            BIN_NAME,
            BIN_NAME.replace('-', "__")
        ));

        assert!(script.contains("        gnome-randr)\n            cmd=\"gnome__randr\""));
        assert!(script.contains("cmd=\"gnome__randr\""));
        assert!(script.contains("        gnome__randr)"));
        assert!(script.contains("        gnome__randr__modify)"));
    }

    #[test]
    fn zsh_augmentation_supports_autoload_and_eval() {
        let script = augment_zsh("_gnome-randr() {\n}\n_gnome-randr \"$@\"\n".to_string());

        assert!(script.contains("__gnome_randr_static() {\n}"));
        assert!(script.contains("dynamic_output=$(command gnome-randr __complete \"$PREFIX\" \"${prior_words[@]}\" 2>/dev/null)"));
        assert!(script.contains("if [[ -n $dynamic_output ]]; then"));
        assert!(script.contains("if [[ \"${funcstack[1]-}\" == \"_gnome-randr\" || \"${funcstack[1]-}\" == \"_gnome_randr\" ]]; then"));
        assert!(script.contains("compdef _gnome-randr gnome-randr"));
    }
}
