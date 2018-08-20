package main

import (
	"fmt"
	"os"
	"os/exec"
)

var loadedEnvVars = map[string]string{}

const usage = `usage:

Place a ".env" file at the same level where the current working directory is,
then execute dotenv [command] [args...].

The command will be executed, stdin and stdout will be piped, and the exit code
will be passed to your terminal.`

func main() {
	var (
		command string
		args    []string
	)

	switch len(os.Args) {
	case 0, 1:
		errexit("%s", usage)

	case 2:
		command = os.Args[1]
		args = make([]string, 0)

	default:
		command = os.Args[1]
		args = os.Args[2:]
	}

	if err := loadVirtualEnv(".env"); err != nil {
		if _, isNotExist := err.(*FileNotFound); !isNotExist {
			errexit("Can't load variables: %s", err.Error())
		}
	}

	ev := make([]string, 0, len(os.Environ())+len(loadedEnvVars))
	ev = append(ev, os.Environ()...)

	for k, v := range loadedEnvVars {
		ev = append(ev, fmt.Sprintf("%s=%s", k, v))
	}

	cmd := exec.Command(command, args...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Env = ev

	if err := cmd.Run(); err != nil {
		if e, ok := err.(*exec.ExitError); ok {
			os.Exit(e.ExitCode())
		}

		errexit("Unable to execute command %q: %s", command, err.Error())
	}
}

func errexit(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, "[dotenv] "+format+"\n", args...)
	os.Exit(1)
}
