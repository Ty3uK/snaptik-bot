package mp4

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"mime"
	"net/http"
	"path/filepath"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/platform"
)

const maxMetaSize = 32 * 1024

type FetchResponse struct {
	Resolution *Resolution
	Meta       *[]byte
	Body       io.ReadCloser
}

var replacer = strings.NewReplacer(" ", "%20", "&amp;", "&")

func Fetch(ctx context.Context, httpClient *http.Client, sourceUrl string, targetPlatform platform.Platform) (*FetchResponse, error) {
	// Workaround for partially encoded query params
	sourceUrl = replacer.Replace(sourceUrl)

	req, err := http.NewRequestWithContext(ctx, "GET", sourceUrl, nil)
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %s", err)
	}
	switch targetPlatform {
	case platform.PlatformTikTok:
		req.Header.Add("Origin", "https://www.tiktok.com")
		req.Header.Add("Referer", "https://www.tiktok.com")
	}

	res, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %s", err)
	}

	contentType := detectContentType(res)
	if contentType != "" && contentType != "video/mp4" && contentType != "application/octet-stream" {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Bad content type: %s", contentType)
	}

	contentLength := res.ContentLength
	if contentLength == 0 {
		_ = res.Body.Close()
		return nil, errors.New("Empty Content-Length")
	}

	if contentLength >= 50*1024*1024 {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Video is larger than 50MB")
	}

	metaSize := min(maxMetaSize, contentLength)
	meta := make([]byte, metaSize)
	n, err := io.ReadFull(res.Body, meta)
	if err != nil {
		fmt.Printf("%+v\n", res)
		_ = res.Body.Close()
		return nil, fmt.Errorf("Cannot read meta from body: %s", err)
	}
	if int64(n) != metaSize {
		_ = res.Body.Close()
		return nil, fmt.Errorf("Read more bytes than expected: %d expected, got %d", metaSize, n)
	}

	if contentType == "" {
		contentType = http.DetectContentType(meta)
	}
	if contentType != "video/mp4" && contentType != "application/octet-stream" {
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

func detectContentType(res *http.Response) string {
	contentType := res.Header.Get("Content-Type")
	if contentType != "" && contentType != "application/octet-stream" {
		return contentType
	}

	contentDisposition := res.Header.Get("Content-Disposition")
	if contentDisposition != "" {
		_, params, _ := mime.ParseMediaType(contentDisposition)
		filename := params["filename"]
		if filename != "" {
			ext := filepath.Ext(filename)
			mtype := mime.TypeByExtension(ext)
			if mtype != "" {
				return mtype
			}
		}
	}

	return ""
}
