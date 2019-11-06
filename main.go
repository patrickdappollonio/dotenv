package main

import (
	"os"
	"os/exec"
	"path/filepath"
)

var (
	dotenvLocations = envOrDefault("DOTENV_FOLDER_PATH", "~/.dotenv/")
	dotenvUse       = envOrDefault("DOTENV", "")
)

const usage = `Usage: dotenv [--environment | -e path] [command] [args...]

Place a ".env" file at the same level where the current working directory is,
then execute dotenv [command] [args...].

Additionally, use a ".env" file from ~/.dotenv/ or wherever $DOTENV_FOLDER_PATH
points to, by specifying $DOTENV or --environment=filename or -e=filename (without
the extension) and it will be used automatically.

The command will be executed, stdin, stdout and stderr will be piped, and the
exit code will be passed to your terminal.`

func main() {
	var (
		command string
		evfile  string
	)

	args := os.Args[1:]

	if len(os.Args) <= 1 {
		errexit("missing command and/or arguments\n\n%s", usage)
	}

	if isFlagSet("-h", "--help") {
		os.Stdout.WriteString(usage + "\n")
		return
	}

	if dotenvUse != "" {
		evfile = filepath.Join(dotenvLocations, dotenvUse+".env")
	}

	if isFlagSet("--environment", "-e") {
		vals := getFlagValue("--environment", "-e")
		venv := ""

		if v, found := vals["--environment"]; found {
			venv = v
		}

		if v, found := vals["-e"]; found {
			if venv != "" {
				errexit("Both flags provided: --environment and -e -- must specify only one")
			}

			venv = v
		}

		evfile = filepath.Join(dotenvLocations, venv+".env")
		args = getAllArgsAfter(venv)
	}

	switch len(args) {
	case 0:
		errexit("missing command and/or arguments\n\n%s", usage)

	case 1:
		command = args[0]
		args = []string{}

	default:
		command = args[0]
		args = args[1:]
	}

	if evfile == "" {
		evfile = ".env"
	}

	envvars, err := loadVirtualEnv(evfile)
	if err != nil {
		if _, ok := err.(*FileNotFound); ok {
			errexit("No dotenv file found at %q", evfile)
		}

		errexit("Can't read environment variable file: %s", err.Error())
	}

	cmd := getCommand(command, args...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Env = append(os.Environ(), envvars...)

	if err := cmd.Run(); err != nil {
		if e, ok := err.(*exec.ExitError); ok {
			os.Exit(e.ExitCode())
		}

		errexit("Unable to execute command %q: %s", command, err.Error())
	}
}
