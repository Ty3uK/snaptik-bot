FROM golang:1.23-alpine AS builder
WORKDIR /app
ENV CGO_ENABLED=1
COPY cmd/ /app/cmd
COPY internal/ /app/internal
COPY test/ /app/test
COPY go.sum go.mod /app

RUN \
    apk add --no-cache --update git build-base && \
    go mod tidy && \
    go build ./cmd/snaptik-bot


FROM alpine:latest AS runner
WORKDIR /app
RUN apk --no-cache add ca-certificates tzdata libc6-compat libgcc libstdc++
COPY --from=builder /app/snaptik-bot /app/snaptik-bot
CMD ["/app/snaptik-bot"]
