package resolvers

import "context"

type Resolver interface {
	ResolveUrl(ctx context.Context, sourceUrl string) (*string, error)
}
