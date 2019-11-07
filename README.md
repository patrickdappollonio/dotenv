# dotenv

Usage: `dotenv [--environment | -e path] [command] [args...]`

Place a ".env" file at the same level where the current working directory is,
then execute `dotenv [command] [args...]`.

Additionally, use a ".env" file from `~/.dotenv/` or wherever `$DOTENV_FOLDER_PATH`
points to, by specifying `$DOTENV` or `--environment=filename` or `-e=filename` (without
the extension) and it will be used automatically. If the path passed is absolute,
then whatever file passed will be used as environment if it can be parsed as a
`key=value` format.

The command will be executed, stdin, stdout and stderr will be piped, and the
exit code will be passed to your terminal.