use std::io::BufWriter;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::{cli::Cli, tapo::server::rpc::Device};

const DEVICE_COMPLETION_COMMANDS: [&str; 6] = [
    "set",
    "info",
    "usage",
    "on",
    "off",
    "reset"
];

/// Save device names in cache so that shell completions can use them
pub fn save_device_completions(devices: &Vec<Device>) {
    let path = dirs::cache_dir().unwrap_or(
        dirs::home_dir().unwrap_or_default().join(".cache")
    );

    let contents = devices.iter().map(|d| d.name.clone()).collect::<Vec<String>>().join(" ");
    if let Err(err) = std::fs::write(path.join("tapoctl-device-completions.txt"), contents) {
        println!("Error whilst saving device completions: {err}");
    }
}

/// Generate the shell completions for a given shell and binary
pub fn generate_completions(shell: Shell, binary: &str) -> String {
    let mut buf = BufWriter::new(Vec::new());
    generate(shell, &mut Cli::command(), binary, &mut buf);

    let mut output =         String::from_utf8_lossy(&buf.into_inner().unwrap_or(Vec::new())).to_string();
    if shell.eq(&Shell::Bash) {
        for command in DEVICE_COMPLETION_COMMANDS {
            let search = format!("{binary}__{})", command.replace(" ", "__")); // case in big switch

            if let Some(pos) = output
                .find(&search)
                .and_then(|pos| output[pos..].find("opts=").map(|n| pos + n))
                .and_then(|pos| output[pos..].find("\n").map(|n| pos + n + 1))
            {
                output.insert_str(
                    pos,
                    &device_completion_bash(command.split(" ").count() as u32 + 1),
                );
            }
        }
    }

    output.to_string()
}

pub fn device_completion_bash(level: u32) -> String {
    let base = r#"
        if [[ ${cur} != -* %%CONDITIONS%% ]] ; then
            path="$XDG_CACHE_HOME"
            [[ -n "$path" ]] || path="$HOME/.cache"
            path="$path/tapoctl-device-completions.txt"

            devices="<DEVICE>"
            [[ -e "$path" ]] && devices="$(cat $path)"

            COMPREPLY=( $(compgen -W "$devices %%OPTIONS%%" -- "${cur}") )
            return 0
        fi
    "#;

    base
        .replace("%%CONDITIONS%%", &format!("&& ${{COMP_CWORD}} -eq {level}"))
        .replace("%%OPTIONS%%", "")
}