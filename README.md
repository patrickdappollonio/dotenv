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

## Installation

[Download the binary from the Releases page](https://github.com/patrickdappollonio/dotenv/releases)
and place the binary in a place in your `$PATH`. Then simply call `dotenv` with whatever
configuration needed.

## ... but why?

`dotenv` comes as a solution to a problem I was running pretty often. I do a lot of
terminal stuff and some CLIs use files to configure themselves while others use a
combination of an environment variable that somehows configure the rest of the CLI via
a file.

As an example, the `openstack` CLI uses `OS_CLOUD` to define a "cloud configuration" which
is then taken from the file at `~/.config/openstack/clouds.yaml` which can define multiple
clouds. I'm always changing which cloud I use, and I realize `openstack` can also be
configured with environment variables such as:

```bash
export OS_AUTH_URL=<url-to-openstack-identity>
export OS_PROJECT_NAME=<project-name>
export OS_USERNAME=<user-name>
export OS_PASSWORD=<password>  # (optional)
```

So rather than doing that, I created a few files in `~/.dotenv/` which point to a specific
cloud configuration, say `~/.dotenv/cloud1.env`, `~/.dotenv/cloud2.env`. They all contain
the same set of environment variables but with different values, plus, they contain a single
`DOTENV_COMMAND` variable which is always set to `openstack`, like this:

```
DOTENV_COMMAND=openstack
OS_AUTH_URL=<url-to-openstack-identity>
OS_PROJECT_NAME=<project-name>
OS_USERNAME=<user-name>
OS_PASSWORD=<password>  # (optional)
```

Now, depending on what cloud I want to execute commands, like getting servers (`openstack server list`)
or loadbalancers (`openstack loadbalancer list`), I simply do:

```bash
$ dotenv -e=cloud1 server list
$ dotenv -e=cloud2 loadbalancer list
```

Or I can even alias the commands to work a bit faster:

```bash
alias os1='dotenv -e=cloud1'
alias os2='dotenv -e=cloud2'
```

I realize though that, at the end, we come down to managing multiple files rather than just one
(as in, we went from one `clouds.yaml` to 2 `.env` files), but the solution is highly scriptable
and oftentimes the `clouds.yaml` file is being edited to support new clouds (I work with quite a
few usernames, passwords and cloud endpoints).

Other more commonly used feature is that I store some `.env` file in the local directory where I'm
developing a Go web server which needs to have environment variables that I can't post to Github,
like `$PORT` or even `$SMTP_USERNAME`. This is the easiest to solve with `dotenv` because of this:

```bash
$ cat .env
PORT=8081
SMTP_HOST=localhost
SMTP_USER=patrick
SMTP_PASS=demo

$ dotenv go run *.go
Server listening on port 8081...
```

## Adding new features?

I welcome any Pull Request. The license of this software is also permissive enough that
you can make it yours and / or use it in corporate environments. The sky is the limit!

Most of the code here has been written in a rush, so assume typos are a thing. You can
also see there's a lack of testing, so if you're into that, send me a PR! No PR is
useless when it comes down to this app.