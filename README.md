# cargo-exec [![Crates.io](https://img.shields.io/crates/v/cargo-alias-exec)](https://crates.io/crates/cargo-alias-exec)[![License](https://img.shields.io/github/license/squirreljetpack/cargo-exec)](https://github.com/squirreljetpack/cargo-exec/blob/main/LICENSE)

A tiny script extending cargo aliases to arbitrary commands.

Something like git aliases and npm run, in cargo.

If you have the completion setup for it, you can also see your aliases with their definitions:

![cargo completions](screen.png)

# Steps

`cargo install cargo-alias-exec` (or `cargo install --path .` if cloning).

Add the following to `$PROJECT_DIR/.cargo/config.toml` or `~/.cargo/config.toml` ([See](https://doc.rust-lang.org/cargo/reference/config.html))

```toml
[alias]
# prefix your command with 'exec'
butter = ['exec', 'sh', '-c', 'cargo build && cargo test && RUST_LOG=debug cargo run']
tea = ['exec', 'sh', '-c', 'cargo insta test; cargo insta review']

# no need: cargo-exec is for non-cargo commands
crumpets = 'clippy -- --allow warnings'
```

then run `cargo butter`.

# Arguments

You can execute an inline shell script using the `-s` flag:

```toml
[alias]
once = ['exec', '-s', 'zsh', 'rustc $1 && ./${1%.*}']
```

and then invoke with `cargo once script.rs`.

Or directly:
```shell
> cargo exec -s zsh "echo \$1" hi
hi
```

When `-s` is absent, environment variables in your arguments are evaluated before being passed to the main command. This is a convenience provided to make it easier to define aliases in cases like the following:

```toml
[alias]
toast = "exec cargo run -i brioche -o $HOME/counter/" # to prevent this behavior, escape $ like so: \$HOME
```

# Environment variables
For your convenience, a few environment variables are set inside the shell:


- `CARGO_PREFIX` finds the nearest ancestor directory with `Cargo.toml`
- `_LEFT_ARGS` and `_RIGHT_ARGS` contain the input arguments split at `--`, and are useful when wrapping other cargo subcommands:
```toml
cc = ['exec', '-s', 'sh', 'if [ -e "$CARGO_PREFIX/clippy.toml" ]; then eval cargo clippy "$_LEFT_ARGS" -- -A clippy::uninlined_format_args "$_RIGHT_ARGS"; else cargo check; fi']
# Now (if you have the IDE extension), you can tell rust-analyzer to use cc as your custom cargo-check command 
# Note: The use of eval is because your arguments inside _LEFT_ARGS are escaped for whitespace-safety
#       LEFT_ARGS and RIGHT_ARGS are the non-escaped variants, buyer beware!
```

You can also set your own environment variables preceding all arguments:
- Additionally, the working directory can be set with `PWD`, where relative paths are resolved with respect to `CARGO_PREFIX`.
```toml
[alias]
"@mytask" = "exec PWD=examples cargo run compose.yaml -t quadlet -o outputs"
```


# FAQ

**Why under [aliases] instead of a dedicated tasks section (like npm run)?**

Another approach would put the definition somewhere like:

```toml
[tool.cargo-exec]
task = ["echo", "Hello"]
```
But:
1. Although completions for tasks could be defined for cargo-exec, `cargo exec` wouldn't propogate them. Even if they were supported, getting completions for your tasks would involve an extra step during installation.
2. cargo aliases already implements the correct definition of "tasks" from the hierarchical parsing of config files, getting to offload as much logic as possible seems a good thing to me.
3. If you're worried about confusing your aliases with your tasks, you can assign your tasks a prefix, like `@mytask`.
