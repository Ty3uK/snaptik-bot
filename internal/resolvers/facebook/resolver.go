package facebook

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

var downloadLinkRegex = regexp.MustCompile(`id="(hdlink|sdlink)".+?href="(.+?)"`)

type FacebookResolver struct {
	httpClient *http.Client
}

func NewFacebookResolver(httpClient *http.Client) resolvers.Resolver {
	return &FacebookResolver{
		httpClient: httpClient,
	}
}

func (r *FacebookResolver) ResolveUrl(ctx context.Context, sourceUrl string) (*string, error) {
	form := url.Values{}
	form.Set("URLz", sourceUrl)

	req, err := http.NewRequestWithContext(ctx, "POST", "https://www.fdown.net/download.php", strings.NewReader(form.Encode()))
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %s", err)
	}

	req.Header.Add("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
	req.Header.Set("Referer", "https://www.fdown.net")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %s", err)
	}
	defer res.Body.Close()

	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read body: %s", err)
	}

	matches := downloadLinkRegex.FindSubmatch(body)
	if len(matches) == 0 {
		return nil, fmt.Errorf("Cannot match regexp")
	}

	var result string
	if len(matches) == 2 {
		result = string(matches[1])
	} else {
		result = string(matches[2])
	}

	return &result, nil
}
