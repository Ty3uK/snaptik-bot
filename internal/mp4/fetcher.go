package mp4

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
)

const metaSize = 32 * 1024

type FetchResponse struct {
	Resolution *Resolution
	Meta       *[]byte
	Body       io.ReadCloser
}

func Fetch(ctx context.Context, httpClient *http.Client, sourceUrl string) (*FetchResponse, error) {
	// Workaround for partially encoded query params
	sourceUrl = strings.ReplaceAll(sourceUrl, " ", "%20")

	req, err := http.NewRequestWithContext(ctx, "GET", sourceUrl, nil)
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %s", err)
	}

	res, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %s", err)
	}

	contentLength := res.ContentLength
	if contentLength == 0 {
		_ = res.Body.Close()
		return nil, errors.New("Empty Content-Length")
	}

	contentLength = contentLength / 1024 / 1024
	if contentLength >= 50 {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Video is larger than 50MB")
	}

	meta := make([]byte, metaSize)
	n, err := io.ReadFull(res.Body, meta)
	if err != nil {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Cannot read meta from body: %s", err)
	}
	if n != metaSize {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Read more bytes than expected: %d expected, got %d", metaSize, n)
	}

	contentType := res.Header.Get("Content-Type")
	if contentType != "video/mp4" {
		contentType = http.DetectContentType(meta)
	}
	if contentType != "video/mp4" {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Bad content type: %s", contentType)
	}

	resolution, err := ParseResolution(bytes.NewReader(meta))
	if err != nil || resolution == nil {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Cannot parse resolution: %s", err)
	}

	return &FetchResponse{
		Resolution: resolution,
		Meta:       &meta,
		Body:       res.Body,
	}, nil
}
