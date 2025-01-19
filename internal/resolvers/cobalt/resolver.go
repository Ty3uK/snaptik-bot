package cobalt

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
)

type CobaltResolver struct {
	httpClient *http.Client
}

type CobaltRequest struct {
	Url          string `json:"url"`
	VideoQuality string `json:"videoQuality"`
	TiktokH265   bool   `json:"tiktokH265"`
}

type CobaltResponse struct {
	Status   string  `json:"status"`
	Error    *string `json:"error"`
	Url      *string `json:"url"`
	Filename *string `json:"filename"`
}

func NewCobaltResolver(httpClient *http.Client) resolvers.Resolver {
	return &CobaltResolver{
		httpClient: httpClient,
	}
}

func (r *CobaltResolver) ResolveUrl(ctx context.Context, sourceUrl string) (*string, error) {
	body, err := json.Marshal(CobaltRequest{
		Url:          sourceUrl,
		VideoQuality: "max",
		TiktokH265:   true,
	})
	if err != nil {
		return nil, fmt.Errorf("Cannot encode request body: %s", err)
	}

	req, err := http.NewRequestWithContext(ctx, "POST", "http://cobalt:9000", bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %s", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")

	res, err := r.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Response error: %s", err)
	}

	if res.StatusCode != 200 {
		return nil, fmt.Errorf("Response error: %s", res.Status)
	}

	var result CobaltResponse
	defer res.Body.Close()
	err = json.NewDecoder(res.Body).Decode(&result)
	if err != nil {
		return nil, fmt.Errorf("Cannot decode response body: %s", err)
	}
	if result.Error != nil {
		return nil, fmt.Errorf("Bad response from service: %s", *result.Error)
	}
	if result.Status != "redirect" && result.Status != "tunnel" {
		return nil, fmt.Errorf("Unsupported status: %s", result.Status)
	}

	return result.Url, nil
}
