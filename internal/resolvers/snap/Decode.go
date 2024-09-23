package snap

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
)

func Decode(h string, _ int, n string, t uint64, e int, _ int) (string, error) {
	var result strings.Builder
	replaces := []string{}

	for i := 0; i < len(h); i++ {
		s := ""
		for i < len(h) && h[i] != n[e] {
			s += string(h[i])
			i++
		}

		replaces = []string{}
		for j := 0; j < len(n); j++ {
			replaces = append(replaces, string(n[j]), fmt.Sprint(j))
		}
		s = strings.NewReplacer(replaces...).Replace(s)

		if !unicode.IsDigit(rune(s[0])) {
			result.WriteString(s)
			continue
		}
		p, err := strconv.ParseUint(s, e, 0)
		if err != nil {
			return "", err
		}
		result.WriteRune(rune(p - t))
	}

	return result.String(), nil
}
