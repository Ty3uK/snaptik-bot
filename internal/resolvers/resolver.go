package resolvers

type Resolver interface {
	ResolveUrl(sourceUrl string) (*string, error)
}
