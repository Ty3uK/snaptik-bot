package resolvers

type Resolver interface {
	ResolveUrl(url string) (*string, error)
}
