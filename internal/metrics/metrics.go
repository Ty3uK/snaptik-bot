package metrics

import (
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var HttpRequestsTotal = promauto.NewCounter(prometheus.CounterOpts{
	Name: "bot_http_requests_total",
	Help: "The total number of processed requests",
})
