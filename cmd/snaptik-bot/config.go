package main

import (
	"errors"
	"os"
)

type Config struct {
	WebhookUrl    string
	BotToken      string
	DbPath        string
	ListenAddress string
}

func parseConfig() (*Config, error) {
	webhookUrl := os.Getenv("WEBHOOK_URL")
	if webhookUrl == "" {
		return nil, errors.New("WEBHOOK_URL environment variable is not set.")
	}

	botToken := os.Getenv("BOT_TOKEN")
	if botToken == "" {
		return nil, errors.New("BOT_TOKEN environment variable is not set.")
	}

	dbPath := os.Getenv("DB_PATH")
	if botToken == "" {
		dbPath = "./db.sqlite3"
	}

	listenAddress := os.Getenv("LISTEN_ADDRESS")
	if botToken == "" {
		listenAddress = ":8080"
	}

	return &Config{
		WebhookUrl:    webhookUrl,
		BotToken:      botToken,
		DbPath:        dbPath,
		ListenAddress: listenAddress,
	}, nil
}
