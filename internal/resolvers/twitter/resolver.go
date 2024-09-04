package twitter

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"regexp"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

var DOWNLOAD_LINK_REGEXP = regexp.MustCompile(`<a.+?href=\\?"(.+?)\\?"`)

type twitterResponse struct {
	Status  string  `json:"status"`
	Data    *string `json:"data"`
	Message *string `json:"msg"`
}

type TwitterResolver struct {
	httpClient *http.Client
}

func NewTwitterResolver(httpClient *http.Client) resolvers.Resolver {
	return &TwitterResolver{
		httpClient: httpClient,
	}
}

func (r *TwitterResolver) ResolveUrl(sourceUrl string) (*string, error) {
	form := url.Values{}
	form.Set("q", sourceUrl)
	form.Set("lang", "en")

	req, err := http.NewRequest("POST", "https://savetwitter.net/api/ajaxSearch", strings.NewReader(form.Encode()))
	if err != nil {
		return nil, fmt.Errorf("Cannot create new request: %e", err)
	}

	req.Header.Add("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
	req.Header.Add("Referer", "https://savetwitter.net/")
	req.Header.Add("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Response error: %e", err)
	}
	if res.StatusCode != 200 {
		return nil, fmt.Errorf("Response error: %s", res.Status)
	}

	defer res.Body.Close()
	var data twitterResponse
	err = json.NewDecoder(res.Body).Decode(&data)
	if err != nil {
		return nil, fmt.Errorf("Cannot unmarshal response body: %e", err)
	}

	if data.Message != nil {
		return nil, fmt.Errorf("Bad response %s", *data.Message)
	}

	matches := DOWNLOAD_LINK_REGEXP.FindStringSubmatch(*data.Data)
	if len(matches) < 2 {
		return nil, fmt.Errorf("Cannot find link: %s", *data.Data)
	}

	result := strings.Clone(matches[1])
	return &result, nil
}
