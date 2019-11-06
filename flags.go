package main

import "os"

func isFlagSet(flag ...string) bool {
	for i, args := 0, os.Args[1:]; i < len(args); i++ {
		for j := 0; j < len(flag); j++ {
			if x := len(flag[j]); x <= len(args[i]) && args[i][:x] == flag[j] {
				return true
			}

			if args[i] == flag[j] {
				return true
			}
		}
	}
	return false
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
