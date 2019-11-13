package main

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"os"
	"os/user"
	"path/filepath"
	"strings"
)

type filenotfound struct {
	name string
}

func (e *filenotfound) Error() string {
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
			return nil, &filenotfound{name: tmplfile}
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

func startswith(s, prefix string) bool {
	return len(s) >= len(prefix) && s[0:len(prefix)] == prefix
}

func expand(path string) (string, error) {
	if !startswith(path, "~/") {
		return path, nil
	}

	usr, err := user.Current()
	if err != nil {
		return path, err
	}
	return filepath.Join(usr.HomeDir, path[1:]), nil
}

func loadVirtualEnv(fp string) (map[string]string, error) {
	if fp == "" {
		return nil, nil
	}

	fp, err := expand(fp)
	if err != nil {
		return nil, fmt.Errorf("unable to expand %q in path: %s", "~", err.Error())
	}

	data, err := loadFile(fp)
	if err != nil {
		return nil, err
	}

	ev := make(map[string]string)
	sc := bufio.NewScanner(data)

	for sc.Scan() {
		k, v := parseLine(sc.Text())
		if k == "" || v == "" {
			continue
		}

		ev[k] = v
	}

	return ev, nil
}

func parseLine(line string) (string, string) {
	if startswith(strings.TrimSpace(line), "#") {
		return "", ""
	}

	items := strings.Split(line, "=")
	if len(items) < 2 {
		return "", ""
	}

	return strings.ToUpper(items[0]), strings.Join(items[1:], "=")
}

func envOrDefault(key, defval string) string {
	if v, found := os.LookupEnv(key); found {
		if s := strings.TrimSpace(v); s != "" {
			return s
		}
	}

	return defval
}

func errexit(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, "[dotenv] "+format+"\n", args...)
	os.Exit(1)
}

func envFilePresentInHome(filename string) (string, bool) {
	filename = filepath.Join(dotenvLocations, filename+".env")
	filename, _ = expand(filename)

	info, err := os.Stat(filename)
	if os.IsNotExist(err) {
		return "", false
	}

	return filename, !info.IsDir()
}
