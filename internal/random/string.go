package random

import (
	"crypto/rand"
	"fmt"
	"math/big"
)

var dict = []rune("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_-")
var dictSize = big.NewInt(int64(len(dict)))

func GetRandomString(n int) (*string, error) {
	runes := make([]rune, n)
	for i := 0; i < n; i++ {
		index, err := rand.Int(rand.Reader, dictSize)
		if err != nil {
			return nil, fmt.Errorf("Cannot create random string: %s", err)
		}
		runes[i] = dict[index.Int64()]
	}
	result := string(runes)
	return &result, nil
}
