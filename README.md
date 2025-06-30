# cargo-exec

A tiny script for defining cargo aliases for arbitrary commands.

Something like git aliases and npm run, in cargo.

If you have the completion setup for it, you can also see your aliases with their definitions:

![cargo completions](screen.png)

# Steps

`cargo install cargo-exec-subcommand` (or `cargo install --path .` if cloning).

Add the following to `$PROJECT_DIR/.cargo/config.toml` or `~/.cargo/config.toml` ([See](https://doc.rust-lang.org/cargo/reference/config.html))

```toml
[alias]
# prefix your command with 'exec'
butter = ['exec', 'sh', '-c', 'cargo build && cargo test && RUST_LOG=debug cargo run']
toast = ['exec', 'sh', '-c', 'cargo insta test; cargo insta review']

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

and then invoke with `cargo one script.rs`.

Or directly:
```shell
> cargo exec -s zsh "echo \$1" hi
hi
```

# FAQ

**Why under [aliases] instead of a dedicated tasks section (like npm run)?**

Another approach would put the definition in something like:

```
[tool.cargo-exec]
task = ["echo", "Hello"]

```
But:
1. Completions can be defined for cargo-exec, but `cargo exec` wouldn't propogate them. Even if it were supported, getting completions for your tasks would involve an extra step during installation.
2. cargo aliases already implements the correct definition of "tasks" from the hierarchical parsing of config files, getting to offload as much logic as possible seems a good thing to me.
3. If you're worried about confusing your aliases with your tasks, you can assign your tasks a prefix, like `.mytask`.

