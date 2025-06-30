use std::env;
use std::path::PathBuf;
use std::process::{Command, exit};
#[cfg(not(windows))]
use std::os::unix::process::CommandExt;
use regex::Regex;

fn find_cargo_prefix() -> Option<PathBuf> {
    let mut current = env::current_dir().ok()?;
    loop {
        if current.join("Cargo.toml").is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}


fn replace_shell_vars(input: &str) -> String {
    let re = Regex::new(r"(?P<escape>\\)?\$(?P<var>[A-Za-z_][A-Za-z0-9_]*|\{[^}]+\})").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        if caps.name("escape").is_some() {
            format!("${}", &caps["var"])
        } else {
            let var = &caps["var"];
            let key = if var.starts_with('{') && var.ends_with('}') {
                &var[1..var.len()-1]
            } else {
                var
            };
            env::var(key).unwrap_or_default()
        }
    })
    .into_owned()
}

fn main() {
    const SEPARATOR: &str = "--";


    let mut args = env::args().skip(1).peekable();

    if args.peek().map(String::as_str) == Some("exec") {
        args.next();
    }

    let argv: Vec<String> = args.collect();

    if argv.is_empty() {
        eprintln!("Error: Must specify command to execute");
        exit(1);
    }

    let mut command = if argv[0] == "-s" {
        if argv.len() < 3 {
            eprintln!("Error: -s requires a shell and a command");
            exit(1);
        }
        let shell = &argv[1];
        let script = &argv[2];
        let script_args = &argv[3..];

        let split_index = script_args.iter().position(|arg| arg == SEPARATOR).unwrap_or(script_args.len());

        let left_args = &script_args[..split_index];
        let right_args = if split_index < script_args.len() { &script_args[split_index + 1..] } else { &[] };

        let left_env = left_args.join(" ");
        let right_env = right_args.join(" ");

        let left_env_escaped = left_args
            .iter()
            .map(|s| format!("'{}'", s.replace('\'', r"'\''")))
            .collect::<Vec<String>>()
            .join(" ");

        let right_env_escaped = right_args
            .iter()
            .map(|s| format!("'{}'", s.replace('\'', r"'\''")))
            .collect::<Vec<String>>()
            .join(" ");

        let mut command = Command::new(shell);
        command.arg("-c");
        command.arg(format!("f() {{ {script}; }}; f \"$@\""));
        command.arg("_"); // dummy $0
        command.args(script_args);
        command.env("LEFT_ARGS", left_env);
        command.env("RIGHT_ARGS", right_env);
        command.env("_LEFT_ARGS", left_env_escaped);
        command.env("_RIGHT_ARGS", right_env_escaped);
        command
    } else {
        let mut command = Command::new(&argv[0]);
        let replaced_args: Vec<String> = argv[1..].iter()
            .map(|arg| replace_shell_vars(arg))
            .collect();

        command.args(&replaced_args);
        command
    };

    if let Some(prefix) = find_cargo_prefix() {
        command.env("CARGO_PREFIX", prefix);
    }

    #[cfg(not(windows))]
    {
        let err = command.exec();
        eprintln!("Could not exec: {err}")
    }

    #[cfg(windows)]
    {
        match command.status() {
            Ok(status) => {
                if status.success() {
                    exit(0);
                } else {
                    exit(status.code().unwrap_or(1));
                }
            },
            Err(e) => eprintln!("Could not spawn: {e}"),
        }
    }

    exit(1);
}