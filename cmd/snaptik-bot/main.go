package main

import (
	"crypto/tls"
	"encoding/json"
	"log"
	"net/http"
	"net/url"
	"os"

	_ "net/http/pprof"

	"github.com/Ty3uK/snaptik-bot/internal/db"
	"github.com/Ty3uK/snaptik-bot/internal/mp4"
	"github.com/Ty3uK/snaptik-bot/internal/platform"
	"github.com/Ty3uK/snaptik-bot/internal/random"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/shorts"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/snap"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/twitter"
	"github.com/Ty3uK/snaptik-bot/internal/telegram"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

var START_MESSAGE = `
Hi! 👋

I'm a bot that can download video from:

- TikTok
- Instagram
- Youtube Shorts

Just send me a link and I'll send a video back to you 💪


Source code: https://github.com/Ty3uK/snaptik-bot

Creator: @xxxTy3uKxxx
`

func main() {
	config := zap.NewProductionConfig()
	config.EncoderConfig.EncodeTime = zapcore.ISO8601TimeEncoder
	config.Encoding = "console"
	zapLogger, _ := config.Build()
	defer func() {
		err := zapLogger.Sync()
		if err != nil {
			log.Print(err)
		}
	}()
	logger := zapLogger.Sugar()

	webhookUrl := os.Getenv("WEBHOOK_URL")
	if webhookUrl == "" {
		logger.Fatalln("No `WEBHOOK_URL` environment variable is found.")
	}
	parsedWebhookUrl, err := url.Parse(webhookUrl)
	if err != nil {
		logger.Fatalf("Cannot parse webhook url: %s", err)
	}

	botToken := os.Getenv("BOT_TOKEN")
	if botToken == "" {
		logger.Fatalln("No `BOT_TOKEN` environment variable is found.")
	}

	listenAddress := os.Getenv("LISTEN")
	if listenAddress == "" {
		listenAddress = ":8080"
	}

	dbPath := os.Getenv("DB_PATH")
	if dbPath == "" {
		dbPath = "./db.sqlite3"
	}

	logger.Infoln("Opening database.")
	dbClient, err := db.NewDbClient(dbPath)
	if err != nil {
		logger.Fatalf("Cannot create db client: %s", err)
	}
	defer dbClient.Close()

	httpClient := http.Client{
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{
				CurvePreferences: []tls.CurveID{tls.CurveP256, tls.CurveP384, tls.CurveP521, tls.X25519},
			},
		},
	}

	logger.Infoln("Generating secret token.")
	secretToken, err := random.GetRandomString(64)
	if err != nil {
		logger.Errorf("Cannot create secret token: %s", err)
	}

	tgClient := telegram.NewTelegramClient(botToken, &httpClient, logger)

	logger.Infoln("Settings webhook.")
	res, err := tgClient.SetWebook(&telegram.SetWebhook{
		Url:         webhookUrl,
		SecretToken: secretToken,
	})
	if err != nil {
		logger.Fatalf("Cannot set webhook: %s", err)
	}
	if !*res {
		logger.Fatalf("Cannot set webhook", *res)
	}

	http.HandleFunc(parsedWebhookUrl.Path, func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}

		if secretToken != nil && r.Header.Get("X-Telegram-Bot-Api-Secret-Token") != *secretToken {
			logger.Errorln("Cannot validate X-Telegram-Bot-Api-Secret-Token")
			w.WriteHeader(http.StatusOK)
			return
		}

		defer r.Body.Close()
		var update telegram.Update
		err := json.NewDecoder(r.Body).Decode(&update)
		if err != nil {
			logger.Errorf("Cannot decode message: %s", err)
		}

		message := update.Message
		if message == nil {
			logger.Errorln("update.message == nil")
			w.WriteHeader(http.StatusOK)
			return
		}

		chat := message.Chat
		if chat == nil {
			logger.Errorln("update.message.chat == nil")
			w.WriteHeader(http.StatusOK)
			return
		}

		if message.Text == nil {
			logger.Errorw("update.message.text == nil", "message", message)
			w.WriteHeader(http.StatusOK)
			return
		}
		messageText := *message.Text

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
				logger.Errorf("Cannot send message: %s", err)
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
			logger.Errorw("Cannot send message", "error", err)
			w.WriteHeader(http.StatusOK)
			return
		}

		SendBadUrlMessage := func() {
			_, err := tgClient.EditMessageText(&telegram.EditMessageText{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
				Text:      "❌ Only TikTok, Instagram, Twitter or Shorts links are accepted.",
			})
			if err != nil {
				logger.Errorf("Cannot edit message: %s", err)
			}
		}

		SendCannotProcessVideoMessage := func() {
			_, err := tgClient.EditMessageText(&telegram.EditMessageText{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
				Text:      "❌ Cannot process video.",
			})
			if err != nil {
				logger.Errorf("Cannor edit message: %s", err)
			}
		}

		DeleteMessage := func() {
			_, err = tgClient.DeleteMessage(&telegram.DeleteMessage{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
			})
			if err != nil {
				logger.Errorf("Cannot delete message: %s", err)
			}
		}

		parsedUrl, err := url.Parse(messageText)
		if err != nil {
			logger.Errorf("Cannot parse url: %s", err)
			SendBadUrlMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		savedVideo, err := dbClient.GetVideo(messageText)
		if err != nil {
			logger.Errorf("Cannot get video from db: %s", err)
		}
		if savedVideo != nil {
			_, err := tgClient.SendVideo(&telegram.SendVideo{
				ChatId:           chat.Id,
				Video:            savedVideo.FileId,
				ReplyToMessageId: message.MessageId,
				Caption:          &messageText,
			})
			if err != nil {
				logger.Errorf("Cannot send video: %s", err)
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
			logger.Errorf("Cannot parse platform from url: %s", messageText)
			SendBadUrlMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		var resolver resolvers.Resolver
		switch parsedPlatform {
		case platform.PlatformTikTok, platform.PlatformInstagram:
			resolver = snap.NewSnapResolver(&httpClient, parsedPlatform)
		case platform.PlatformShorts:
			resolver = shorts.NewShortsResolver(&httpClient)
		case platform.PlatformTwitter:
			resolver = twitter.NewTwitterResolver(&httpClient)
		default:
			logger.Errorf("Unrechable code: %+v", parsedPlatform)
			w.WriteHeader(http.StatusOK)
			return
		}

		targetUrl, err := resolver.ResolveUrl(parsedUrl.String())
		if err != nil {
			logger.Errorw("Cannot resolve url", "error", err, "source_url", messageText)
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		res, err := mp4.Fetch(&httpClient, logger, *targetUrl)
		if err != nil {
			logger.Errorw("Cannot parse target url", "error", err, "source_url", messageText, "target_url", *targetUrl)
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
			logger.Errorw("Cannot send video", "error", err, "source_url", messageText, "target_url", *targetUrl)
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		DeleteMessage()

		if video != nil && video.Video != nil {
			_, err = dbClient.InsertVideo(messageText, video.Video.FileId)
			if err != nil {
				logger.Errorw("Cannot insert video to db: %s", "error", err, "source_url", messageText, "file_id", video.Video.FileId)
			}
		}

		w.WriteHeader(http.StatusOK)
	})

	logger.Infof("Starting server at %s", listenAddress)
	logger.Fatal(http.ListenAndServe(listenAddress, nil))
}
