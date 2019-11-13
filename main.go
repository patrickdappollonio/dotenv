package main

import (
	"io/ioutil"
	"log"
	"os"
	"os/exec"
)

const (
	aliasKey  = "DOTENV_COMMAND"
	strictKey = "DOTENV_STRICT"
	debugKey  = "DOTENV_DEBUG"
)

var (
	dotenvLocations = envOrDefault("DOTENV_FOLDER_PATH", "~/.dotenv/")
	dotenvUse       = envOrDefault("DOTENV", "")
	dotenvStrict    = envOrDefault(strictKey, "")
	version         = "development"

	knownDotenvVars = [...]string{"DOTENV_FOLDER_PATH", "DOTENV", debugKey, strictKey, aliasKey}
)

const usage = `Usage: dotenv [--environment | -e path] [command] [args...]

Place a ".env" file at the same level where the current working directory is,
then execute dotenv [command] [args...].

Additionally, use a ".env" file from ~/.dotenv/ or wherever $DOTENV_FOLDER_PATH
points to, by specifying $DOTENV or --environment=filename or -e=filename (without
the extension) and it will be used automatically. If the path passed is absolute,
then whatever file passed will be used as environment if it can be parsed as a
key=value format.

If the dotenv file sets an environment variable named DOTENV_COMMAND whose value
is a valid, runnable command, the command will be used and all the remaining
arguments will be sent to the command. For example, the following call will execute
"kubectl get pods"

	$ cat ~/.dotenv/kubectl.env
	DOTENV_COMMAND=kubectl
	KUBECONFIG=/home/patrick/.kube/cluster.yaml

	$ dotenv -e=kubectl get pods
	# since the command is already set in the dotenv file, you
	# don't need to specify it like "dotenv -e=kubectl kubectl get pods"

If $DOTENV_STRICT is set to any value, and set either through environment variables
or in the environment variables file, strict mode is applied, where the command
gets executed only with the environment variables from the environment file, and
without the environment variables from the environment. This mode is useful to not
leak environment variables to your commmands that don't really need them, but also
keep in mind some programs rely on $PATH to be set, or $HOME or other useful
environment variables.

A cool example with no arguments but configuration given via environment variables:

	$ DOTENV=<(echo -e "DOTENV_COMMAND=env\nNAME=joe\nDOTENV_STRICT=1") dotenv
	NAME=joe

dotenv will execute your command, stdin, stdout and stderr will be piped, and the
exit code will be passed to your terminal.`

func main() {
	logger := log.New(ioutil.Discard, "[dotenv-debug] ", log.Lshortfile|log.LstdFlags)

	if os.Getenv(debugKey) != "" {
		logger.SetOutput(os.Stdout)
	}

	var (
		command string
		evfile  string
	)

	args := os.Args[1:]

	if isControlFlagSet("-h", "--help") {
		os.Stdout.WriteString(usage + "\n")
		return
	}

	if isControlFlagSet("-v", "--version") {
		os.Stdout.WriteString("[dotenv] version " + version + "\n")
		return
	}

	if dotenvUse != "" {
		logger.Printf("environment variable $DOTENV set to: %q -- using that as the file", dotenvUse)
		evfile = dotenvUse
	}

	if isControlFlagSet("--environment", "-e") {
		vals := getFlagValue("--environment", "-e")
		venv := ""

		logger.Printf("environment parameters parsed: %v", vals)

		if v, found := vals["--environment"]; found {
			logger.Printf("long parameter --environment set to: %q", v)
			venv = v
		}

		if v, found := vals["-e"]; found {
			if venv != "" {
				logger.Printf("exiting because both flags, --environment and -e were provided")
				errexit("Both flags provided: --environment and -e -- must specify only one")
			}

			logger.Printf("short parameter -e set to: %q", v)
			venv = v
		}

		if startswith(venv, "/") || startswith(venv, "./") {
			logger.Printf("environment file passed %q starts with a control character, assuming full path", venv)
			evfile = venv
		} else {
			if fp, found := envFilePresentInHome(venv); found {
				logger.Printf("found a file in the user's directory with the file name matching %q: %s", venv, fp)
				evfile = fp
			} else {
				logger.Printf("no file found in user's directory for %q, assuming full path", venv)
				evfile = venv
			}
		}

		args = getAllArgsAfter(venv)
		logger.Printf("parsed arguments after environment flags to be: %#v", args)
	}

	if evfile == "" {
		logger.Printf("no env file set, defaulting to assuming there's one in the current directory")
		evfile = ".env"
	}

	envvars, err := loadVirtualEnv(evfile)
	if err != nil {
		if _, ok := err.(*filenotfound); ok {
			logger.Printf("unable to find dotenv file at %q", evfile)
			errexit("No dotenv file found at %q", evfile)
		}

		logger.Printf("unknown error while handling envfile %q: %s", evfile, err.Error())
		errexit("Can't read environment variable file: %s", err.Error())
	}

	aliascmd, hasalias := envvars[aliasKey]
	logger.Printf("found alias in env file? %v -- alias: %q", hasalias, aliascmd)

	switch len(args) {
	case 0:
		if !hasalias {
			logger.Printf("exiting just because no alias was set and no commands were passed")
			errexit("missing command and / or arguments, see --help")
		}

	case 1:
		command = args[0]
		args = []string{}

	default:
		command = args[0]
		args = args[1:]
	}

	logger.Printf("got command %q -- args: %#v", command, args)

	if hasalias {
		if command != "" {
			args = append([]string{command}, args...)
		}

		command = aliascmd
		delete(envvars, aliasKey)

		logger.Printf("swapping command due to alias to %q -- args: %#v", command, args)
	}

	if strict, found := envvars[strictKey]; found {
		dotenvStrict = strict
		delete(envvars, strictKey)
	}

	environ := make([]string, 0, len(os.Environ()))
	for _, v := range os.Environ() {
		known := false
		for _, m := range knownDotenvVars {
			if startswith(v, m+"=") {
				known = true
			}
		}

		if !known {
			logger.Printf("Adding unknown env var %q", v)
			environ = append(environ, v)
		}
	}

	vars := make([]string, 0, len(envvars)+len(environ))

	logOffset := 0
	if dotenvStrict == "" {
		logger.Printf("strict mode environment variable not set: appending all current environment variables")
		vars = append(vars, environ...)
		logOffset = len(environ)
	}

	for k, v := range envvars {
		known := false
		for _, m := range knownDotenvVars {
			if m == v {
				known = true
			}
		}

		if !known {
			vars = append(vars, k+"="+v)
		}
	}

	logger.Printf("environment variables to be injected to command (besides %d current env vars): %v", len(environ), vars[logOffset:])

	cmd := getCommand(command, args...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Env = vars

	logger.Printf("command to be executed: %s %v", command, args)

	if err := cmd.Run(); err != nil {
		if e, ok := err.(*exec.ExitError); ok {
			logger.Printf("command exited with exit code: %v", e)
			os.Exit(e.ExitCode())
		}

		logger.Printf("unable to execute command %q: %s", command, err.Error())
		errexit("Unable to execute command %q: %s", command, err.Error())
	}
}
