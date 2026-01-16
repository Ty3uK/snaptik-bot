package tiktok

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strings"

	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

type DataForRehydration struct {
	DefaultScope *struct {
		VideoDetail *struct {
			ItemInfo *struct {
				ItemStruct *struct {
					Video *struct {
						BitrateInfo *[]struct {
							CodecType *string `json:"CodecType"`
							PlayAddr  *struct {
								UrlList *[]string `json:"UrlList"`
							} `json:"PlayAddr"`
						} `json:"bitrateInfo"`
						PlayAddr *string `json:"playAddr"`
					} `json:"video"`
				} `json:"itemStruct"`
			} `json:"itemInfo"`
		} `json:"webapp.video-detail"`
	} `json:"__DEFAULT_SCOPE__"`
}

var DATA = regexp.MustCompile(`<script.*?id="__UNIVERSAL_DATA_FOR_REHYDRATION__".*?>(.*?)<\/script>`)

type TikTokResolver struct {
	httpClient *http.Client
}

func NewTikTokResolver(httpClient *http.Client) resolvers.Resolver {
	return &TikTokResolver{
		httpClient: httpClient,
	}
}

func (r *TikTokResolver) ResolveUrl(ctx context.Context, sourceUrl string) (*string, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", sourceUrl, nil)
	if err != nil {
		return nil, fmt.Errorf("Cannot create new request: %e", err)
	}

	req.Header.Add("Referer", "https://tiktok.com/")
	req.Header.Add("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Response error: %e", err)
	}
	if res.StatusCode != 200 {
		return nil, fmt.Errorf("Response error: %s", res.Status)
	}

	url, err := url.Parse("https://tiktok.com")
	if err != nil {
		return nil, fmt.Errorf("Cannot parse url: %e", err)
	}
	r.httpClient.Jar.SetCookies(url, res.Cookies())

	defer res.Body.Close()
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("Cannot read body: %e", err)
	}

	matches := DATA.FindSubmatch(body)
	if len(matches) < 2 {
		return nil, fmt.Errorf("Cannot find __UNIVERSAL_DATA_FOR_REHYDRATION__")
	}

	var data DataForRehydration
	err = json.Unmarshal(matches[1], &data)
	if err != nil {
		return nil, fmt.Errorf("Cannot unmarshal __UNIVERSAL_DATA_FOR_REHYDRATION__: %e", err)
	}

	if data.DefaultScope == nil || data.DefaultScope.VideoDetail == nil || data.DefaultScope.VideoDetail.ItemInfo == nil || data.DefaultScope.VideoDetail.ItemInfo.ItemStruct == nil || data.DefaultScope.VideoDetail.ItemInfo.ItemStruct.Video == nil || data.DefaultScope.VideoDetail.ItemInfo.ItemStruct.Video.BitrateInfo == nil || len(*data.DefaultScope.VideoDetail.ItemInfo.ItemStruct.Video.BitrateInfo) == 0 {
		return nil, fmt.Errorf("Bad __UNIVERSAL_DATA_FOR_REHYDRATION__")
	}

	videoUrl := data.DefaultScope.VideoDetail.ItemInfo.ItemStruct.Video.PlayAddr
	for _, video := range *data.DefaultScope.VideoDetail.ItemInfo.ItemStruct.Video.BitrateInfo {
		if strings.Contains(*video.CodecType, "h265") {
			videoUrl = &(*video.PlayAddr.UrlList)[0]
		}
	}

	return videoUrl, nil
}
