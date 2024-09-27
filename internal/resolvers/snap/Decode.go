package snap

import (
	"strconv"
	"strings"
	"unicode"
)

func Decode(h string, _ int, n string, t uint64, e int, _ int) (string, error) {
	var result strings.Builder
	result.Grow(len(h))

	replaces := make([]string, 0, len(n)*2)
	for j := 0; j < len(n); j++ {
		replaces = append(replaces, string(n[j]), strconv.Itoa(j))
	}
	replacer := strings.NewReplacer(replaces...)

	var sb strings.Builder
	for i := 0; i < len(h); i++ {
		sb.Reset()

		for i < len(h) && h[i] != n[e] {
			sb.WriteByte(h[i])
			i++
		}

		s := replacer.Replace(sb.String())

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
