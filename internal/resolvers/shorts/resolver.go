package shorts

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"sort"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

var CSRF_REGEXP = regexp.MustCompile(`<input.+?name="csrf_token".+?value="(.+)".+?>`)
var JSON_REGEXP = regexp.MustCompile(`(?s)set_listener\(.+?(\[.+\]).+?"a"`)

type authData struct {
	csrf    string
	cookies []*http.Cookie
}

type media struct {
	FormatNote string `json:"format_note"`
	Url        string `json:"url"`
}

type ShortsResolver struct {
	httpClient *http.Client
}

func NewShortsResolver(httpClient *http.Client) resolvers.Resolver {
	return &ShortsResolver{
		httpClient: httpClient,
	}
}

func (r *ShortsResolver) ResolveUrl(sourceUrl string) (*string, error) {
	authData, err := r.getAuthData()
	if err != nil {
		return nil, fmt.Errorf("Cannot get auth data: %e", err)
	}

	form := url.Values{}
	form.Set("csrf_token", authData.csrf)
	form.Set("url", sourceUrl)

	req, err := http.NewRequest("POST", "https://shortsmate.com/en/download", strings.NewReader(form.Encode()))
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %e", err)
	}

	req.Header.Add("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
	req.Header.Set("Referer", "https://shortsmate.com/en/download")

	for _, cookie := range authData.cookies {
		req.AddCookie(cookie)
	}

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Response error: %e", err)
	}

	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read response body: %e", err)
	}

	url, err := r.getMediaUrl(string(body))
	if err != nil {
		return nil, fmt.Errorf("Cannot get media url: %e", err)
	}

	return url, nil
}

func (r *ShortsResolver) getAuthData() (*authData, error) {
	req, err := http.NewRequest("GET", "https://shortsmate.com/en/", nil)
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %e", err)
	}

	req.Header.Set("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
	req.Header.Set("Referer", "https://shortsmate.com/en/download")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot get auth data: %e", err)
	}

	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read auth data response body: %e", err)
	}

	matches := CSRF_REGEXP.FindSubmatch(body)
	if len(matches) < 2 {
		return nil, fmt.Errorf("Cannot find `csrf`: %s", body)
	}

	csrf := string(matches[1])
	return &authData{cookies: res.Cookies(), csrf: csrf}, nil
}

func (r *ShortsResolver) getMediaUrl(html string) (*string, error) {
	matches := JSON_REGEXP.FindStringSubmatch(html)
	if len(matches) < 2 {
		return nil, fmt.Errorf("Cannot find json in html: %s", html)
	}

	data := "[" + matches[1] + "]"

	var outerList [][]media
	err := json.Unmarshal([]byte(data), &outerList)
	if err != nil {
		return nil, fmt.Errorf("Cannot unmarshal: %e", err)
	}
	if len(outerList) == 0 || len(outerList[0]) == 0 {
		return nil, fmt.Errorf("Empty formats outerList")
	}

	list := outerList[0]
	sort.SliceStable(list, func(i, j int) bool {
		if list[i].FormatNote == "1080p" || list[i].FormatNote == "720p" {
			return true
		}
		if list[i].FormatNote == "1080p" && list[j].FormatNote == "720p" {
			return true
		}
		return false
	})

	return &list[0].Url, nil
}
