package snap

import (
	"os"
	"testing"
)

func TestDecode(t *testing.T) {
	type Args struct {
		u int
		n string
		t uint64
		e int
		r int
	}
	type Test struct {
		name   string
		input  string
		output string
		args   Args
	}
	tests := []Test{
		{
			name:   "TikTok",
			input:  "../../../test/snap/decode_tiktok_input.txt",
			output: "../../../test/snap/decode_tiktok_output.txt",
			args:   Args{4, "JyivhHWLV", 9, 8, 20},
		},
		{
			name:   "Instagram",
			input:  "../../../test/snap/decode_insta_input.txt",
			output: "../../../test/snap/decode_insta_output.txt",
			args:   Args{20, "RSHywucEs", 49, 3, 20},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			input, err := os.ReadFile(tc.input)
			if err != nil {
				t.Fatal(err)
			}

			output, err := os.ReadFile(tc.output)
			if err != nil {
				t.Fatal(err)
			}

			result, err := Decode(string(input), tc.args.u, tc.args.n, tc.args.t, tc.args.e, tc.args.r)
			if err != nil {
				t.Fatal(err)
			}

			if result != string(output) {
				t.Fatalf("Expected %v, got %v", string(output), result)
			}
		})
	}
}
