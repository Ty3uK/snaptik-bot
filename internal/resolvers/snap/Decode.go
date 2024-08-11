package snap

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
)

func Decode(h string, _ int, n string, t uint64, e int, _ int) (string, error) {
	result := ""

	for i := 0; i < len(h); i++ {
		s := ""
		for i < len(h) && h[i] != n[e] {
			s += string(h[i])
			i++
		}
		for j := 0; j < len(n); j++ {
			s = strings.ReplaceAll(s, string(n[j]), fmt.Sprint(j))
		}
		if !unicode.IsDigit(rune(s[0])) {
			result += s
			continue
		}
		p, err := strconv.ParseInt(s, e, 0)
		if err != nil {
			return "", err
		}
		result += string(rune(p - int64(t)))
	}

	return result, nil
}
