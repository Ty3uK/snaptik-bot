package mp4

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"strings"
)

type FetchResponse struct {
	Resolution *Resolution
	Meta       *[]byte
	Body       io.ReadCloser
}

func Fetch(httpClient *http.Client, sourceUrl string) (*FetchResponse, error) {
	// Workaround for partially encoded query params
	sourceUrl = strings.ReplaceAll(sourceUrl, " ", "%20")

	res, err := http.Get(sourceUrl)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %s", err)
	}

	meta := make([]byte, 512)
	_, err = res.Body.Read(meta)
	if err != nil {
		return nil, fmt.Errorf("Cannot read meta from body: %s", err)
	}

	contentType := http.DetectContentType(meta)
	if contentType != "video/mp4" {
		return nil, fmt.Errorf("Bad content type: %s", contentType)
	}

	resolution, err := ParseResolution(bytes.NewReader(meta))
	if err != nil || resolution == nil {
		return nil, fmt.Errorf("Cannot parse resolution: %s", err)
	}

	return &FetchResponse{
		Resolution: resolution,
		Meta:       &meta,
		Body:       res.Body,
	}, nil
}
