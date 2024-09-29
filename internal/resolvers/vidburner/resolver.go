package vidburner

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"sort"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

var tokenRegexp = regexp.MustCompile(`<input.+?type="hidden".+?value="(.+?)"`)

type Response struct {
	Medias *[]struct {
		Url     string `json:"url"`
		Quality string `json:"quality"`
		Size    int    `json:"size"`
	} `json:"medias"`
}

type VidBurnerResolver struct {
	httpClient *http.Client
}

func NewVidBurnerResolver(httpClient *http.Client) resolvers.Resolver {
	return &VidBurnerResolver{
		httpClient: httpClient,
	}
}

func (r *VidBurnerResolver) ResolveUrl(ctx context.Context, sourceUrl string) (*string, error) {
	token, err := r.getToken(ctx)
	if err != nil {
		return nil, fmt.Errorf("Cannot get token: %s", err)
	}

	form := url.Values{}
	form.Set("url", sourceUrl)
	form.Set("token", *token)

	req, err := http.NewRequestWithContext(ctx, "POST", "https://vidburner.com/wp-json/aio-dl/video-data/", strings.NewReader(form.Encode()))
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %s", err)
	}

	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	req.Header.Add("Referer", "https://vidburner.com/")
	req.Header.Add("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %s", err)
	}
	defer res.Body.Close()

	var data Response
	err = json.NewDecoder(res.Body).Decode(&data)
	if err != nil {
		return nil, fmt.Errorf("Cannot decode body: %s", err)
	}
	if data.Medias == nil || len(*data.Medias) == 0 {
		return nil, errors.New("Cannot find any media")
	}

	sort.SliceStable(*data.Medias, func(i, j int) bool {
		return (*data.Medias)[i].Size > (*data.Medias)[j].Size
	})

	result := strings.Clone((*data.Medias)[0].Url)
	return &result, nil
}

func (r *VidBurnerResolver) getToken(ctx context.Context) (*string, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", "https://vidburner.com/", nil)
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %s", err)
	}

	req.Header.Add("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %s", err)
	}
	defer res.Body.Close()

	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read body: %s", err)
	}

	matches := tokenRegexp.FindSubmatch(body)
	if len(matches) == 0 {
		return nil, fmt.Errorf("Cannot find token: %s", string(body))
	}

	result := string(matches[0])
	return &result, nil
}
