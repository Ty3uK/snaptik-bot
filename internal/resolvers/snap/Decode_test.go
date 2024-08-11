package snap

import (
	"os"
	"testing"
)

func TestDecodeTikTok(t *testing.T) {
	input, err := os.ReadFile("../../../test/snap/decode_tiktok_input.txt")
	if err != nil {
		t.Fatal(err)
	}

	output, err := os.ReadFile("../../../test/snap/decode_tiktok_output.txt")
	if err != nil {
		t.Fatal(err)
	}

	result, err := Decode(string(input), 4, "JyivhHWLV", 9, 8, 20)
	if err != nil {
		t.Fatal(err)
	}

	if result != string(output) {
		t.Fatalf("Expected %v, got %v", string(output), result)
	}
}

func TestDecodeInstagram(t *testing.T) {
	input, err := os.ReadFile("../../../test/snap/decode_insta_input.txt")
	if err != nil {
		t.Fatal(err)
	}

	output, err := os.ReadFile("../../../test/snap/decode_insta_output.txt")
	if err != nil {
		t.Fatal(err)
	}

	result, err := Decode(string(input), 20, "RSHywucEs", 49, 3, 20)
	if err != nil {
		t.Fatal(err)
	}

	if result != string(output) {
		t.Fatalf("Expected %v, got %v", string(output), result)
	}
}
