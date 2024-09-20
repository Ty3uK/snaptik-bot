package mp4

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"strconv"
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

	contentLengthStr := res.Header.Get("Content-Length")
	contentLength, err := strconv.Atoi(contentLengthStr)
	if err != nil {
		return nil, fmt.Errorf("Cannot parse Content-Length: %s", err)
	}
	if contentLength > 50 * 1024 * 1024 {
		return nil, fmt.Errorf("Video is larger than 50MB")
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
