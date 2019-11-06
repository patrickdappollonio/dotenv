// +build linux

package main

import (
	"os/exec"
	"syscall"
)

func getCommand(command string, args ...string) *exec.Cmd {
	cmd := exec.Command(command, args...)
	cmd.SysProcAttr = &syscall.SysProcAttr{Pdeathsig: syscall.SIGTERM}

	return cmd
}
