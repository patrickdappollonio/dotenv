package main

import "os"

func isControlFlagSet(flag ...string) bool {
	if len(os.Args) <= 1 {
		return false
	}

	found := false

	for _, v := range flag {
		if v == os.Args[1] {
			found = true
			break
		}

		if startswith(os.Args[1], v+"=") {
			found = true
			break
		}
	}

	return found
}

func getFlagValue(keys ...string) map[string]string {
	out := make(map[string]string)

	if len(keys) == 0 {
		return out
	}

	args := os.Args[1:]
	for pos, arg := range args {
		if len(arg) > 0 && arg[0] != '-' {
			continue
		}

		for _, name := range keys {
			if prefix := name + "="; len(arg) >= len(prefix) && arg[:len(prefix)] == prefix {
				key := arg[:len(prefix)-1]
				value := arg[len(prefix):]
				out[key] = value
				continue
			}

			if arg == name {
				if next := pos + 1; next < len(args) {
					nextval := args[next]

					if nextval != "" && nextval[0] == '-' {
						continue
					}

					out[name] = nextval
					continue
				}
			}
		}
	}

	return out
}

func getAllArgsAfter(value string) []string {
	args := os.Args[1:]
	for pos, v := range args {
		if len(v) >= len(value) && v[len(v)-len(value):] == value {
			return append([]string{}, args[pos+1:]...)
		}
	}

	return nil
}
