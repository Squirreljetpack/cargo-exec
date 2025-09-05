use regex::Regex;
use std::collections::HashMap;
use std::env;
#[cfg(not(windows))] use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{exit, Command};
use std::iter::Peekable;
use std::str::Chars;

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

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};

fn join_os_strings(args: &[OsString]) -> OsString {
    let mut result = OsString::new();
    let mut first = true;
    for arg in args {
        if !first {
            result.push(" ");
        }
        result.push(arg);
        first = false;
    }
    result
}

fn quote_shell_args(args: &[OsString]) -> Result<String, &'static str> {
    let mut result = String::new();

    for item in args {
        let s = item.to_str().ok_or("argument not valid UTF-8")?;
        if !result.is_empty() {
            result.push(' ');
        }
        result.push('"');
        result.push_str(&s.replace('"', "\\\""));
        result.push('"');
    }

    Ok(result)
}

#[cfg(unix)]
fn contains_equal(s: &OsStr) -> Option<(OsString, OsString)> {
    let bytes = s.as_bytes();
    bytes.iter().position(|&b| b == b'=').map(|idx| {
        let (lhs_bytes, rhs_bytes) = bytes.split_at(idx);
        let lhs = OsString::from_vec(lhs_bytes.to_vec());
        let rhs = OsString::from_vec(rhs_bytes[1..].to_vec());
        (lhs, rhs)
    })
}

#[cfg(windows)]
fn contains_equal(s: &OsStr) -> Option<(OsString, OsString)> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let wide: Vec<u16> = s.encode_wide().collect();
    if let Some(idx) = wide.iter().position(|&c| c == b'=' as u16) {
        let lhs = OsString::from_wide(&wide[..idx]);
        let rhs = OsString::from_wide(&wide[idx + 1..]);
        Some((lhs, rhs))
    } else {
        None
    }
}

fn err1(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    exit(1);
}

const VAR_REGEX: &str = "[A-Za-z_][A-Za-z0-9_]*";

fn replace_shell_vars<'a>(
    input: &'a OsStr,
    env_vars: &'a HashMap<OsString, OsString>,
) -> Result<String, &'a OsStr> {
    if let Some(input) = input.to_str() {
        let re = Regex::new(&format!(
            r"(?P<escape>\\)?\$(?P<var>{VAR_REGEX}|\{{{VAR_REGEX}\}})"
        ))
        .unwrap();

        Ok(re
            .replace_all(input, |caps: &regex::Captures| {
                if caps.name("escape").is_some() {
                    format!("${}", &caps["var"])
                } else {
                    let var = &caps["var"];
                    let key = if var.starts_with('{') && var.ends_with('}') {
                        OsString::from(&var[1..var.len() - 1])
                    } else {
                        OsString::from(var)
                    };

                    if let Some(value) = env_vars.get(&key) {
                        value.to_string_lossy().into_owned()
                    } else {
                        env::var(&key).unwrap_or_default()
                    }
                }
            })
            .into_owned())
    } else {
        Err(input)
    }
}

struct ShellWords<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> ShellWords<'a> {
    fn new(s: &'a str) -> Self {
        ShellWords {
            chars: s.chars().peekable(),
        }
    }
}

impl<'a> Iterator for ShellWords<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }

        self.chars.peek()?;

        let mut word = String::new();
        let mut in_quotes: Option<char> = None;
        let mut escaped = false;

        for c in self.chars.by_ref() {
            if escaped {
                word.push(c);
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }

            match in_quotes {
                Some(q) if c == q => {
                    in_quotes = None;
                }
                Some(_) => {
                    word.push(c);
                }
                None => match c {
                    '\'' | '"' => {
                        in_quotes = Some(c);
                    }
                    _ => {
                        if c.is_whitespace() {
                            break
                        } else {
                            word.push(c);
                        }
                    }
                },
            }
        }

        if in_quotes.is_some() {
            eprintln!("warning: unclosed quote ignored");
        }

        if escaped {
            eprintln!("warning: dangling escape omitted");
        }

        Some(word)
    }
}

fn parse_shell_words(input: &'_ OsStr) -> Result<ShellWords<'_>, &'static str> {
    let s = input.to_str().ok_or("argument not valid UTF-8")?;
    Ok(ShellWords::new(s))
}

fn usage() {
    println!(
        r#"Usage: cargo exec [<VAR=VALUE>...] [options]... <command> [<args>...]

        Execute arbitrary commands as cargo subcommands.

        Options:
        -s [shell]      Execute the command string by passing to a shell (default: $SHELL).

        -r              Run the command in the project's root directory (where Cargo.toml is).

        -w              Splits the first argument following shell parsing rules.

        -h              Print this help message and exit.

        Arguments:
        <VAR=VALUE>     Sets environment variables.
        <command>       The command to execute.
        <args>...       Arguments to pass to the command.
        "#
    );
    exit(0);
}

fn main() {
    const SEPARATOR: &str = "--";

    let mut args = env::args_os().skip(1).peekable();
    let mut pwd: Option<PathBuf> = None;

    args.next_if(|s| {
        s == OsStr::new("exec")
    });
    // argparse

    let mut shell_opt = false;
    let mut root_opt = false;
    let mut word_opt = false;
    let mut shell: Option<OsString> = None;
    let mut last = None;

    let mut env_vars: HashMap<OsString, OsString> = HashMap::new();

    for arg in args.by_ref() {
        if let Some((key, value)) = contains_equal(&arg) {
            if key == OsStr::new("PWD") {
                pwd = Some(PathBuf::from(value));
            } else if key == OsStr::new("SHELL") {
                shell = Some(value);
            } else {
                env_vars.insert(key, value);
            }
        } else {
            last = Some(arg);
            break;
        }
    }

    let last = last.unwrap_or_else(|| err1("Must specify command to execute"));

    let mut script = if let Some(arg0) = last.to_str().and_then(|s| s.strip_prefix('-')) {
        if arg0 == "--" {
            args.next()
                .unwrap_or_else(|| err1("Must specify command to execute"))
        } else {
            for c in arg0.chars() {
                match c {
                    's' => shell_opt = true,
                    'r' => root_opt = true,
                    'w' => word_opt = true,
                    'h' => usage(),
                    _ => eprintln!("warning: invalid option -{c}, ignoring"),
                }
            }
            args.next()
                .unwrap_or_else(|| err1("Must specify command to execute"))
        }
    } else {
        last
    };

    if shell_opt && word_opt {
        eprintln!("warning: -w has no effect when -s is set");
    }

    let mut command = if shell_opt {
        // get default shell
        let shell = shell.unwrap_or(OsString::from(env::var("SHELL").unwrap_or_else(
            |_| {
                err1("error: -s requires a shell, or $SHELL to be set");
            },
        )));

        let args_vec: Vec<OsString> = args.collect(); // ArgsOs can't be cloned

        // try provide args to script
        script = if let Some(script) = script.to_str() {
            OsString::from(format!("f() {{ {script} ; }}; f \"$@\"",))
        } else {
            if !args_vec.is_empty() {
                eprintln!("warning: Script contains invalid UTF-8, arguments not passed.")
            }
            script
        };

        let mut command = Command::new(shell);
        command.arg("-c");
        command.arg(script);
        command.arg("_"); // dummy $0
        command.args(args_vec.iter());

        // (try) provide left/right_arg env vars
        // if first argument is not a flag, all arguments are right args per standard cargo subcommand behavior.
        let (_left_args, _right_args) = 
            if args_vec
                .first()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with('-'))
            {
                let split_index = args_vec
                    .iter()
                    .position(|arg| arg.to_str() == Some(SEPARATOR))
                    .unwrap_or(args_vec.len());
                (
                    &args_vec[..split_index],
                    if split_index < args_vec.len() {
                        &args_vec[split_index + 1..]
                    } else {
                        &[]
                    },
                )
            } else {
                (&[] as &[OsString], &args_vec[..])
            };

        let left_args = join_os_strings(_left_args);
        let right_args = join_os_strings(_right_args);

        command.env("LEFT_ARGS", left_args);
        command.env("RIGHT_ARGS", right_args);

        if let Ok(escaped) = quote_shell_args(_left_args) {
            command.env("_LEFT_ARGS", escaped);
        }
        if let Ok(escaped) = quote_shell_args(_right_args) {
            command.env("_RIGHT_ARGS", escaped);
        }

        command
    } else if word_opt {
        let mut words = parse_shell_words(&script).unwrap_or_else(|e| err1(e));
        let new_script = if let Some(s) = words.next() {
            s
        } else {
            err1("Must specify command to execute");
        };
        let mut command = Command::new(new_script);
        command.args(words);
        for arg in args {
            if let Ok(arg) = replace_shell_vars(&arg, &env_vars) {
                command.arg(arg);
            } else {
                command.arg(arg);
            }
        }
        command
    } else {
        // run a command directly, replacing variables (replacements fail silently, is there a better way?)
        let mut command = Command::new(script);
        for arg in args {
            if let Ok(arg) = replace_shell_vars(&arg, &env_vars) {
                command.arg(arg);
            } else {
                command.arg(arg);
            }
        }

        command
    };

    command.envs(env_vars);
    
    let original_pwd = env::current_dir();
    let cargo_prefix = find_cargo_prefix();

    // if root_opt, default pwd
    if pwd.is_none() && root_opt {
        if let Some(prefix) = &cargo_prefix {
            pwd = Some(prefix.clone());
        } else {
            eprintln!("warning: Could not find root cargo directory, directory not changed");
        }
    }

    // change_dir to pwd
    if let Some(path) = pwd {
        if path.is_relative() {
            if let Some(ref prefix) = cargo_prefix {
                let path = prefix.join(&path);
                if let Err(e) = env::set_current_dir(&path) {
                    eprintln!(
                        "error: could not change to directory '{}': {}",
                        path.display(),
                        e
                    );
                    exit(1);
                }
            } else {
                err1("error: PWD is relative but CARGO_PREFIX not found.");
            }
        } else if let Err(e) = env::set_current_dir(&path) {
            eprintln!(
                "error: could not change to directory '{}': {}",
                path.display(),
                e
            );
            exit(1);
        }
    }

    // store original: fail silently
    if let Ok(current) = original_pwd {
        command.env("LPWD", current);
    }

    // store cargo_prefix: fail silently
    if let Some(ref prefix) = cargo_prefix {
        command.env("CARGO_PREFIX", prefix);
    }

    #[cfg(not(windows))] {
        let err = command.exec();
        eprintln!("Could not exec {command:?}: {err}")
    }

    #[cfg(windows)] {
        match command.status() {
            Ok(status) => {
                if status.success() {
                    exit(0);
                } else {
                    exit(status.code().unwrap_or(1));
                }
            }
            Err(e) => eprintln!("Could not spawn {command:?}: {e}"),
        }
    }

    exit(1);
}
