package mp4

import (
	"encoding/binary"
	"fmt"
	"io"
)

type Box struct {
	size int64
	name string
}

type Resolution struct {
	Width  uint64
	Height uint64
}

func ParseResolution(reader io.ReadSeeker) (*Resolution, error) {
	var box Box
	buffer := make([]byte, 8)
	for {
		_, err := reader.Read(buffer)
		if err != nil {
			if err == io.EOF {
				return nil, nil
			}
			return nil, fmt.Errorf("Cannot read boxSize: %s", err)
		}
		box.size = int64(binary.BigEndian.Uint32(buffer[0:4]))
		box.name = string(buffer[4:8])
		if box.size == 0 || box.name == "" {
			return nil, fmt.Errorf("Cannot parse box")
		}
		if box.name == "tkhd" {
			resolution, err := parseTkhdBox(reader, &buffer, &box)
			if err != nil {
				return nil, err
			}
			if resolution != nil {
				return resolution, nil
			}
		}
		if box.name == "moov" || box.name == "trak" {
			continue
		}
		_, err = reader.Seek(box.size-8, io.SeekCurrent)
		if err != nil {
			return nil, fmt.Errorf("Cannot seek: %s", err)
		}
	}
}

func parseTkhdBox(reader io.ReadSeeker, buffer *[]byte, box *Box) (*Resolution, error) {
	_, err := reader.Seek(box.size-16, io.SeekCurrent)
	if err != nil {
		return nil, fmt.Errorf("Cannot seek: %s", err)
	}

	_, err = reader.Read(*buffer)
	if err != nil {
		return nil, fmt.Errorf("Cannot read boxSize: %s", err)
	}

	width := uint64(binary.BigEndian.Uint32((*buffer)[0:4]))

	if width != 0 {
		height := uint64(binary.BigEndian.Uint32((*buffer)[4:8]))
		return &Resolution{
			Height: height / 65536,
			Width:  width / 65536,
		}, nil
	}

	return nil, nil
}
