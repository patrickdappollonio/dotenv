# `dotenv`

[![Download](https://img.shields.io/badge/download-here-brightgreen?logo=github)](https://github.com/patrickdappollonio/dotenv/releases) [![Build Status](https://travis-ci.org/patrickdappollonio/dotenv.svg?branch=master)](https://travis-ci.org/patrickdappollonio/dotenv)

Usage: `dotenv [--environment | -e path] [command] [args...]`

Place a `.env` file at the same level where the current working directory is,
then execute `dotenv [command] [args...]`.

Additionally, use a `.env` file from `~/.dotenv/` or wherever `$DOTENV_FOLDER_PATH`
points to, by specifying `$DOTENV` or `--environment=filename` or `-e=filename` (without
the extension) and it will be used automatically. If the path passed is absolute,
then whatever file passed will be used as environment if it can be parsed as a
`key=value` format.

If the `dotenv` file sets an environment variable named `DOTENV_COMMAND` whose value
is a valid, runnable command, the command will be used and all the remaining
arguments will be sent to the command. For example, the following call will execute
`kubectl get pods`

```bash
$ cat ~/.dotenv/kubectl.env
DOTENV_COMMAND=kubectl
KUBECONFIG=/home/patrick/.kube/cluster.yaml

$ dotenv -e=kubectl get pods
# since the command is already set in the dotenv file, you
# don't need to specify it like "dotenv -e=kubectl kubectl get pods"
```

If `$DOTENV_STRICT` is set to any value, and set either through environment variables
or in the environment variables file, strict mode is applied, where the command
gets executed only with the environment variables from the environment file, and
without the environment variables from the environment. This mode is useful to not
leak environment variables to your commmands that don't really need them, but also
keep in mind some programs rely on `$PATH` to be set, or `$HOME` or other useful
environment variables.

A cool example with no arguments but configuration given via environment variables:

```bash
$ DOTENV=<(echo -e "DOTENV_COMMAND=env\nNAME=joe\nDOTENV_STRICT=1") dotenv
NAME=joe
```

`dotenv` will execute your command, `stdin`, `stdout` and `stderr` will be piped, and the
exit code will be passed to your terminal.