// +build !linux

package main

import (
	"os/exec"
)

func getCommand(command string, args ...string) *exec.Cmd {
	return exec.Command(command, args...)
}
