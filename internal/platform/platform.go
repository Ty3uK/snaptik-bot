package platform

import (
	"net/url"
	"strings"
)

type Platform uint

const (
	PlatformUnknown Platform = iota + 1
	PlatformTikTok
	PlatformInstagram
	PlatformShorts
	PlatformTwitter
)

func ParsePlatform(sourceUrl *url.URL) Platform {
	if strings.HasSuffix(sourceUrl.Host, "tiktok.com") {
		return PlatformTikTok
	} else if strings.HasSuffix(sourceUrl.Host, "instagram.com") {
		return PlatformInstagram
	} else if strings.HasSuffix(sourceUrl.Host, "youtube.com") {
		return PlatformShorts
	} else if strings.HasSuffix(sourceUrl.Host, "twitter.com") || strings.HasSuffix(sourceUrl.Host, "x.com") {
		return PlatformTwitter
	}
	return PlatformUnknown
}
