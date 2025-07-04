package main

import (
	"context"
	"crypto/tls"
	"encoding/json"
	"net/http"
	"net/url"
	"os/signal"
	"syscall"
	"time"

	_ "net/http/pprof"

	"github.com/Ty3uK/snaptik-bot/internal/db"
	"github.com/Ty3uK/snaptik-bot/internal/metrics"
	"github.com/Ty3uK/snaptik-bot/internal/mp4"
	"github.com/Ty3uK/snaptik-bot/internal/platform"
	"github.com/Ty3uK/snaptik-bot/internal/random"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/cobalt"
	"github.com/Ty3uK/snaptik-bot/internal/telegram"
	"github.com/prometheus/client_golang/prometheus/promhttp"

	log "github.com/sirupsen/logrus"
)

var START_MESSAGE = `
Hi! 👋

I'm a bot that can download video from:

- TikTok
- Instagram
- Youtube Shorts
- Twitter (X)
- Facebook
- Snaptchat

Just send me a link and I'll send a video back to you 💪


Source code: https://github.com/Ty3uK/snaptik-bot
`

func main() {
	logger := log.New()

	config, err := parseConfig()
	if err != nil {
		logger.WithError(err).Fatal("Cannot parse config")
	}

	ctx, stop := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM, syscall.SIGKILL, syscall.SIGQUIT)
	defer stop()

	parsedWebhookUrl, err := url.Parse(config.WebhookUrl)
	if err != nil {
		logger.WithError(err).Fatal("Cannot parse webhook url.")
	}

	logger.Info("Opening database.")
	dbClient, err := db.NewDbClient(config.DbPath)
	if err != nil {
		logger.WithError(err).Fatal("Cannot create db client")
	}
	go func() {
		<-ctx.Done()
		dbClient.Close()
	}()

	httpClient := http.Client{
		Timeout: time.Second * 30,
		Transport: &http.Transport{
			MaxIdleConns: 300,
			MaxIdleConnsPerHost: 100,
			IdleConnTimeout: time.Second * 30,
			TLSClientConfig: &tls.Config{
				CurvePreferences: []tls.CurveID{tls.CurveP256, tls.CurveP384, tls.CurveP521, tls.X25519},
			},
		},
	}

	logger.Info("Generating secret token.")
	secretToken, err := random.GetRandomString(64)
	if err != nil {
		logger.WithError(err).Error("Cannot create secret token")
	}

	tgClient := telegram.NewTelegramClient(config.BotToken, &httpClient, logger)

	logger.Info("Settings webhook.")
	res, err := tgClient.SetWebook(&telegram.SetWebhook{
		Url:         config.WebhookUrl,
		SecretToken: secretToken,
	})
	if err != nil {
		logger.WithError(err).Fatal("Cannot set webhook")
	}
	if !*res {
		logger.WithError(err).Fatal("Cannot set webhook")
	}

	httpServer := &http.Server{
		Addr: config.ListenAddress,
	}

	http.HandleFunc(parsedWebhookUrl.Path, func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}

		metrics.HttpRequestsTotal.Inc()

		if secretToken != nil && r.Header.Get("X-Telegram-Bot-Api-Secret-Token") != *secretToken {
			log.Error("Cannot validate X-Telegram-Bot-Api-Secret-Token")
			w.WriteHeader(http.StatusOK)
			return
		}

		defer r.Body.Close()
		var update telegram.Update
		err := json.NewDecoder(r.Body).Decode(&update)
		if err != nil {
			log.WithError(err).Error("Cannot decode message")
		}

		message := update.Message
		if message == nil {
			log.Error("update.message is nil")
			w.WriteHeader(http.StatusOK)
			return
		}

		chat := message.Chat
		if chat == nil {
			log.Error("update.message.chat is nil")
			w.WriteHeader(http.StatusOK)
			return
		}

		if message.Text == nil {
			log.Error("update.message.text is nil")
			w.WriteHeader(http.StatusOK)
			return
		}
		messageText := *message.Text

		logger := logger.WithFields(log.Fields{
			"source_url": messageText,
		})

		if messageText == "" {
			w.WriteHeader(http.StatusOK)
			return
		}

		if chat.ChatType != telegram.ChatTypePrivate {
			if messageText != "@SnapTikRsBot" {
				w.WriteHeader(http.StatusOK)
				return
			}

			if message.ReplyToMessage == nil {
				w.WriteHeader(http.StatusOK)
				return
			}

			messageText = *message.ReplyToMessage.Text
		}

		if messageText == "/start" {
			_, err := tgClient.SendMessage(&telegram.SendMessage{
				ChatId: chat.Id,
				Text:   START_MESSAGE,
				LinkPreviewOptions: &telegram.LinkPreviewOptions{
					IsDisabled: true,
				},
			})
			if err != nil {
				logger.WithError(err).Error("Cannot send message")
			}
			w.WriteHeader(http.StatusOK)
			return
		}

		messageToEdit, err := tgClient.SendMessage(&telegram.SendMessage{
			ChatId:           chat.Id,
			Text:             "⏱️  Processing...",
			ReplyToMessageId: message.MessageId,
		})
		if err != nil {
			logger.WithError(err).Error("Cannot send message")
			w.WriteHeader(http.StatusOK)
			return
		}

		SendBadUrlMessage := func() {
			_, err := tgClient.EditMessageText(&telegram.EditMessageText{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
				Text:      "❌ Only TikTok, Instagram, Twitter, Shorts or Facebook links are accepted.",
			})
			if err != nil {
				logger.WithError(err).Error("Cannot edit message")
			}
		}

		SendCannotProcessVideoMessage := func() {
			_, err := tgClient.EditMessageText(&telegram.EditMessageText{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
				Text:      "❌ Cannot process video.",
			})
			if err != nil {
				logger.WithError(err).Error("Cannot edit message")
			}
		}

		DeleteMessage := func() {
			_, err = tgClient.DeleteMessage(&telegram.DeleteMessage{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
			})
			if err != nil {
				logger.WithError(err).Error("Cannot delete message")
			}
		}

		parsedUrl, err := url.Parse(messageText)
		if err != nil {
			logger.WithError(err).Error("Cannot parse url")
			SendBadUrlMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		savedVideo, err := dbClient.GetVideo(messageText)
		if err != nil {
			logger.WithError(err).Error("Cannot get video from db")
		}
		if savedVideo != nil {
			_, err := tgClient.SendVideo(&telegram.SendVideo{
				ChatId:           chat.Id,
				Video:            savedVideo.FileId,
				ReplyToMessageId: message.MessageId,
				Caption:          &messageText,
			})
			if err != nil {
				logger.WithError(err).Error("Cannot send video")
				SendCannotProcessVideoMessage()
				w.WriteHeader(http.StatusOK)
				return
			}

			DeleteMessage()

			w.WriteHeader(http.StatusOK)
			return
		}

		parsedPlatform := platform.ParsePlatform(parsedUrl)
		if parsedPlatform == platform.PlatformUnknown {
			logger.Error("Cannot parse platform")
			SendBadUrlMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		var resolver resolvers.Resolver
		switch parsedPlatform {
		case platform.PlatformTikTok, platform.PlatformInstagram, platform.PlatformFacebook,
			platform.PlatformShorts,
			platform.PlatformTwitter,
			platform.PlatformSnapchat:
			resolver = cobalt.NewCobaltResolver(&httpClient)
		default:
			logger.WithField("platform", parsedPlatform).Error("Unrechable code")
			w.WriteHeader(http.StatusOK)
			return
		}

		targetUrl, err := resolver.ResolveUrl(ctx, parsedUrl.String())
		if err != nil {
			logger.WithError(err).Error("Cannot resolve url")
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		logger = logger.WithField("target_url", *targetUrl)

		res, err := mp4.Fetch(ctx, &httpClient, *targetUrl)
		if err != nil {
			logger.WithError(err).Error("Cannot parse target url")
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		video, err := tgClient.SendVideoFile(&telegram.SendVideoFile{
			ChatId:           chat.Id,
			Video:            &res.Body,
			VideoMeta:        res.Meta,
			ReplyToMessageId: *message.MessageId,
			Caption:          messageText,
			Width:            res.Resolution.Width,
			Height:           res.Resolution.Height,
		})
		if err != nil {
			logger.WithError(err).Error("Cannot send video")
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		DeleteMessage()

		if video != nil && video.Video != nil {
			_, err = dbClient.InsertVideo(messageText, video.Video.FileId)
			if err != nil {
				logger.WithError(err).WithField("file_id", video.Video.FileId).Error("Cannot insert video to db")
			}
		}

		w.WriteHeader(http.StatusOK)
	})

	http.Handle("/metrics", promhttp.Handler())

	// Graceful shutdown
	go func() {
		<-ctx.Done()

		err = httpServer.Shutdown(context.Background())
		if err != nil {
			logger.WithError(err).Fatal("Cannot shutdown server")
		}
	}()

	logger.WithField("address", config.ListenAddress).Info("Starting server")
	err = httpServer.ListenAndServe()
	if err != nil && err != http.ErrServerClosed {
		logger.WithError(err).Error("Failed to listen server")
	}
}
