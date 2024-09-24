package snap

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"regexp"
	"strconv"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/platform"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

var TOKEN_REGEXP = regexp.MustCompile(`<input name="token" value="(.+?)" .+?>`)
var DECODER_ARGS_REGEXP = regexp.MustCompile(`\("(.+?)",(\d+),"(.+?)",(\d+),(\d+),(\d+)\)`)
var RESULT_VIDEO_URL_REGEXP = regexp.MustCompile(`href=\\?"(https://(.*?\.)?(snaptik\.app|snapinsta\.app|rapidcdn\.app)/.*?)\\?"`)

type SnapResolver struct {
	httpClient *http.Client
	platform   platform.Platform
}

func NewSnapResolver(httpClient *http.Client, platform platform.Platform) resolvers.Resolver {
	return &SnapResolver{
		httpClient: httpClient,
		platform:   platform,
	}
}

func (resolver *SnapResolver) ResolveUrl(ctx context.Context, sourceUrl string) (*string, error) {
	token, err := resolver.getToken(ctx)
	if err != nil {
		return nil, fmt.Errorf("Cannot get token: %e", err)
	}

	form := bytes.Buffer{}
	writer := multipart.NewWriter(&form)
	_ = writer.WriteField("url", sourceUrl)
	_ = writer.WriteField("lang", "en")
	_ = writer.WriteField("token", *token)
	_ = writer.WriteField("action", "post")
	err = writer.Close()
	if err != nil {
		return nil, fmt.Errorf("Cannot close multipart.Writer: %e", err)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", resolver.getEndpoint(), bytes.NewReader(form.Bytes()))
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %e", err)
	}
	req.Header.Set("Content-Type", fmt.Sprintf("multipart/form-data; boundary=%s", writer.Boundary()))
	req.Header.Set("Content-Length", fmt.Sprint(form.Len()))
	req.Header.Set("Referer", resolver.getReferer())
	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")

	res, err := resolver.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %e", err)
	}

	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read response body: %e", err)
	}

	matches := DECODER_ARGS_REGEXP.FindStringSubmatch(string(body))
	if len(matches) < 7 {
		return nil, fmt.Errorf("Cannot get decoder args: %s", body)
	}

	h := string(matches[1])
	u, _ := strconv.ParseInt(matches[2], 10, 64)
	n := string(matches[3])
	t, _ := strconv.ParseUint(matches[4], 10, 64)
	e, _ := strconv.ParseInt(matches[5], 10, 64)
	r, _ := strconv.ParseInt(matches[6], 10, 64)
	decoded, err := Decode(h, int(u), n, t, int(e), int(r))
	if err != nil {
		return nil, fmt.Errorf("Decode error: %e", err)
	}

	matches = RESULT_VIDEO_URL_REGEXP.FindStringSubmatch(decoded)
	if len(matches) < 2 {
		return nil, fmt.Errorf("Cannot find result url: %s", decoded)
	}

	result := strings.Clone(matches[1])
	return &result, nil
}

func (r *SnapResolver) getToken(ctx context.Context) (*string, error) {
	referer := r.getReferer()

	req, err := http.NewRequestWithContext(ctx, "GET", referer, nil)
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %e", err)
	}

	req.Header.Set("Host", "snapinsta.app")
	req.Header.Add("Accept", "*/*")
	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:129.0) Gecko/20100101 Firefox/129.0")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot get token: %e", err)
	}

	if res.StatusCode != 200 {
		return nil, fmt.Errorf("Bad status: %s", res.Status)
	}

	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read body: %e", err)
	}

	matches := TOKEN_REGEXP.FindSubmatch(body)
	if len(matches) < 2 {
		return nil, fmt.Errorf("Cannot find token: %s", body)
	}

	result := string(matches[1])
	return &result, nil
}

func (r *SnapResolver) getEndpoint() string {
	switch r.platform {
	case platform.PlatformTikTok:
		return "https://snaptik.app/abc2.php"
	case platform.PlatformInstagram:
		return "https://snapinsta.app/action2.php"
	default:
		return ""
	}
}

func (r *SnapResolver) getReferer() string {
	switch r.platform {
	case platform.PlatformTikTok:
		return "https://snaptik.app/"
	case platform.PlatformInstagram:
		return "https://snapinsta.app/"
	default:
		return ""
	}
}
