use std::collections::HashMap;
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


const VAR_REGEX: &str = "[A-Za-z_][A-Za-z0-9_]*";

fn replace_shell_vars(input: &str) -> String {
    let re = Regex::new(&format!(r"(?P<escape>\\)?\$(?P<var>{VAR_REGEX}|\{{{VAR_REGEX}\}})")).unwrap();
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
    let mut pwd = None;

    if args.peek().map(String::as_str) == Some("exec") {
        args.next();
    }

    let argv: Vec<String> = args.collect();

    let env_var_re = Regex::new(&format!(r"^({VAR_REGEX})=(.*)$")).unwrap();
    let mut env_vars: HashMap<String, String> = HashMap::new();

    // todo: what policy for binaries with '='?
    let mut cmd_index: usize = 0;
    for (i, arg) in argv.iter().enumerate() {
        if let Some(caps) = env_var_re.captures(arg) {
            let key = caps.get(1).unwrap().as_str();
            let value = caps.get(2).unwrap().as_str();
            if key == "PWD" {
                pwd = Some(value.to_string());
            } else {
                env_vars.insert(key.into(), value.into());
            }
            cmd_index = i + 1;
        } else {
            break;
        }
    }

    let argv = &argv[cmd_index..];

    if argv.is_empty() {
        eprintln!("error: Must specify command to execute");
        exit(1);
    }

    let mut command = if argv[0] == "-s" {
        if argv.len() < 3 {
            eprintln!("error: -s requires a shell and a command");
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

    command.envs(env_vars);

    let cargo_prefix = find_cargo_prefix();

    if let Some(pwd) = pwd {
        let path = PathBuf::from(pwd);
        if path.is_relative() {
            if let Some(prefix) = find_cargo_prefix() {
                let path = prefix.join(path);
                if let Err(e) = env::set_current_dir(&path) {
                    eprintln!("error: could not change to directory '{}': {}", path.display(), e);
                    exit(1);
                }
            } else {
                eprintln!("error: PWD is relative but CARGO_PREFIX not found.");
                exit(1);
            }
        } else if let Err(e) = env::set_current_dir(&path) {
            eprintln!("error: could not change to directory '{}': {}", path.display(), e);
            exit(1);
        }
    }

    if let Some(prefix) = cargo_prefix {
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