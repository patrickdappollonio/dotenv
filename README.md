# `dotenv`

**dotenv** is a small command-line utility that allows you to inject environment variables from a `.env` file into a command's environment before running it. It also supports a "strict" mode that only includes variables from the `.env` file plus a few common whitelist of essential environment variables, like `PATH`, `HOME` or even `SHLVL`.

- [`dotenv`](#dotenv)
  - [Features](#features)
  - [Installation](#installation)
    - [Precompiled Binaries](#precompiled-binaries)
    - [Rust and Cargo](#rust-and-cargo)
  - [Usage](#usage)
    - [Loading environment variables](#loading-environment-variables)
    - [From the current working directory](#from-the-current-working-directory)
    - [From a named environment](#from-a-named-environment)
    - [Strict Mode](#strict-mode)
  - [`.env` Format](#env-format)

## Features

- **Automatic `.env` loading:**
  If an `.env` file is present in the current directory, `dotenv` loads it automatically.

- **Named environments:**
  Use `--environment <name>` to load variables from `~/.dotenv/<name>.env`.

- **Strict mode:**
  Use `--strict` to start the command with only the variables from the `.env` file and a minimal whitelist (like `PATH`, `HOME`, etc.).
  The `.env` file itself can enforce strict mode by setting `DOTENV_STRICT=true`.

- **Transparent command execution:**
  After loading the environment variables, `dotenv` executes the specified command, passing all arguments along.

- **Compatibility with commands requiring their own flags:**
  Use a double dash (`--`) to signal that subsequent arguments belong to the executed command, not to `dotenv`.

## Installation

### Precompiled Binaries

Precompiled binaries for Linux, macOS, and Windows are available on the [Releases page](https://github.com/patrickdappollonio/dotenv/releases).

Download the binary for your platform, then move it to a directory in your `PATH`.

### Rust and Cargo

1. Ensure you have [Rust and Cargo](https://www.rust-lang.org/tools/install) installed.
2. Clone this repository:
   ```bash
   git clone https://github.com/yourusername/dotenv.git
   cd dotenv
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```
4. The compiled binary can be found in `target/release/dotenv`.

## Usage

```bash
dotenv [OPTIONS] -- COMMAND [ARGS...]
```

### Loading environment variables

`dotenv` supports two modes of operation: loading environment variables from a `.env` file in the current directory or from a named environment file.

### From the current working directory

By default, `dotenv` loads environment variables from a `.env` file in the current working directory if you specify no arguments.

Consider the following scenario:

```bash
$ cat .env
HELLO=world

$ dotenv -- printenv HELLO
world
```

### From a named environment

If you prefer custom environment variables, you can overwrite `dotenv`'s default `.env` file by specifying a different file. This file however has to come from `dotenv`'s configuration directory, which is `$HOME/.dotenv/`.

Any file here named `<name>.env` can be loaded by specifying `--environment <name>` or `-e <name>`:

```bash
$ cat ~/.dotenv/example.env
FOO=bar

$ dotenv --environment example -- printenv FOO
bar
```

### Strict Mode

Sometimes we might not trust a specific command from wreaking havoc in our environment, and we would rather provide just a limited set of environment variables without exposing the entire environment. This is where strict mode comes in.

A common case scenario might be that you have an environment where you have your AWS credentials stored in the environment variables. You might not want to expose these to a command that you don't trust.

To avoid this, simply run the command with the `--strict` flag:

```bash
$ printenv AWS_ACCESS_KEY_ID
AKIAIOSFODNN7EXAMPLE

$ dotenv --strict -- printenv AWS_ACCESS_KEY_ID
# no output

$ dotenv --strict -- printenv PATH
/usr/local/bin:/usr/bin:/bin
```

The program still received some basic environment variables that are often needed to find other programs, but none of the AWS credentials were exposed.

> [!CAUTION]
> **`dotenv` makes no effort preventing the program to gain access to these environment variables** by other means (like reading configuration files or the untrusted program being able to upload your entire configuration to a remote location).
> It only prevents them from being passed directly to the program.

## `.env` Format

Use simple `KEY=VALUE` lines:

```env
# A comment line
FOO=bar
MYNAME=Alice
DOTENV_STRICT=true
```

- Lines starting with `#` are ignored as comments.
- Trailing comments after `#` on the same line are also ignored, and the lines are space-trimmed.
- Empty lines are ignored.
- Shebangs (`#!`) are ignored and have no effect on how we run the command.
