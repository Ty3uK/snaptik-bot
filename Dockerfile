FROM golang:1.23

WORKDIR /app
COPY cmd/ /app/cmd
COPY internal/ /app/internal
COPY test/ /app/test
COPY go.sum go.mod /app

RUN \
    go mod tidy && \
    go build ./cmd/snaptik-bot

EXPOSE 8080
CMD ["/app/snaptik-bot"]
