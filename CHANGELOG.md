## [0.2.2] - 2025-10-03
- Update README for cargo
## [0.2.0] - 2025-08-31
- Rerelease 0.1.4

## [0.1.4] - 2025-08-29
-  Initial positional arguments are now passed to right_args
-  Initial env var declarations are now substituted
-  -w flag added, which splits single strings.
-  -h for help

## [0.1.3] - 2025-08-04

- Directly parse OsStr arguments for windows compatibility
- various bugfix and refactor

## [0.1.2] - 2025-08-04

- Only change directory with the `-r` flag
- Refactor cli parsing

## [0.1.1] - 2025-08-04

- When using the `-s` flag, if `PWD` is not provided, the working directory is automatically set to `CARGO_PREFIX`.
- If only one argument follows `-s`, it now defaults to `$SHELL`

## [0.1.0] - 2025-07-15

- Initial release
