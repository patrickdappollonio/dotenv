package main

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
)

type FileNotFound struct {
	name string
}

func (e *FileNotFound) Error() string {
	return "file not found: " + e.name
}

func loadFile(fp string) (*bytes.Buffer, error) {
	tmplfile, err := filepath.Abs(fp)
	if err != nil {
		return nil, fmt.Errorf("unable to get path to file %q: %s", fp, err.Error())
	}

	f, err := os.Open(tmplfile)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, &FileNotFound{name: tmplfile}
		}

		return nil, fmt.Errorf("unable to open file %q: %s", fp, err.Error())
	}

	defer f.Close()

	var buf bytes.Buffer
	if _, err := io.Copy(&buf, f); err != nil {
		return nil, fmt.Errorf("unable to read file %q: %s", fp, err.Error())
	}

	return &buf, nil
}

func loadVirtualEnv(fp string) error {
	if fp == "" {
		return nil
	}

	data, err := loadFile(fp)
	if err != nil {
		return err
	}

	sc := bufio.NewScanner(data)
	for sc.Scan() {
		k, v := parseLine(sc.Text())
		if k == "" || v == "" {
			continue
		}

		loadedEnvVars[k] = v
	}

	return nil
}

func parseLine(line string) (string, string) {
	if strings.HasPrefix(strings.TrimSpace(line), "#") {
		return "", ""
	}

	items := strings.Split(line, "=")
	if len(items) < 2 {
		return "", ""
	}

	return strings.ToUpper(items[0]), strings.Join(items[1:], "=")
}
