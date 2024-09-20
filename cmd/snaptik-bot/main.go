package main

import (
	"crypto/tls"
	"encoding/json"
	"log"
	"net/http"
	"net/url"
	"os"

	"github.com/Ty3uK/snaptik-bot/internal/db"
	"github.com/Ty3uK/snaptik-bot/internal/mp4"
	"github.com/Ty3uK/snaptik-bot/internal/platform"
	"github.com/Ty3uK/snaptik-bot/internal/random"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/shorts"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/snap"
	"github.com/Ty3uK/snaptik-bot/internal/resolvers/twitter"
	"github.com/Ty3uK/snaptik-bot/internal/telegram"
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
	webhookUrl := os.Getenv("WEBHOOK_URL")
	if webhookUrl == "" {
		log.Fatalf("No `WEBHOOK_URL` environment variable is found.\n")
	}
	parsedWebhookUrl, err := url.Parse(webhookUrl)
	if err != nil {
		log.Fatalf("Cannot parse webhook url: %s", err)
	}

	botToken := os.Getenv("BOT_TOKEN")
	if botToken == "" {
		log.Fatalf("No `BOT_TOKEN` environment variable is found.\n")
	}

	listenAddress := os.Getenv("LISTEN")
	if listenAddress == "" {
		listenAddress = ":8080"
	}

	dbPath := os.Getenv("DB_PATH")
	if dbPath == "" {
		listenAddress = "./db.sqlite3"
	}

	log.Println("Opening database.")
	dbClient, err := db.NewDbClient(dbPath)
	if err != nil {
		log.Fatalf("Cannot create db client: %s", err)
	}
	defer dbClient.Close()

	httpClient := http.Client{
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{
				CurvePreferences: []tls.CurveID{tls.CurveP256, tls.CurveP384, tls.CurveP521, tls.X25519},
			},
		},
	}

	log.Println("Generating secret token.")
	secretToken, err := random.GetRandomString(64)
	if err != nil {
		log.Printf("Cannot create secret token: %s", err)
	}

	tgClient := telegram.NewTelegramClient(botToken, &httpClient)

	log.Println("Settings webhook.")
	res, err := tgClient.SetWebook(&telegram.SetWebhook{
		Url:         webhookUrl,
		SecretToken: secretToken,
	})
	if err != nil {
		log.Fatalf("Cannot set webhook: %s\n", err)
	}
	if !*res {
		log.Fatalf("Cannot set webhook: %t", *res)
	}

	http.HandleFunc(parsedWebhookUrl.Path, func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			w.WriteHeader(http.StatusMethodNotAllowed)
			return
		}

		if secretToken != nil && r.Header.Get("X-Telegram-Bot-Api-Secret-Token") != *secretToken {
			log.Println("Cannot validate X-Telegram-Bot-Api-Secret-Token")
			w.WriteHeader(http.StatusOK)
			return
		}

		defer r.Body.Close()
		var update telegram.Update
		err := json.NewDecoder(r.Body).Decode(&update)
		if err != nil {
			log.Printf("Cannot decode message: %s\n", err)
		}

		message := update.Message
		if message == nil {
			log.Println("update.message == nil")
			w.WriteHeader(http.StatusOK)
			return
		}

		chat := message.Chat
		if chat == nil {
			log.Println("update.message.chat == nil")
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
				log.Printf("Cannot send message: %s", err)
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
			log.Printf("Cannot send message: %s", err)
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
				log.Printf("Cannot edit message: %s", err)
			}
		}

		SendCannotProcessVideoMessage := func() {
			_, err := tgClient.EditMessageText(&telegram.EditMessageText{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
				Text:      "❌ Cannot process video.",
			})
			if err != nil {
				log.Printf("Cannot edit message: %s", err)
			}
		}

		DeleteMessage := func() {
			_, err = tgClient.DeleteMessage(&telegram.DeleteMessage{
				ChatId:    chat.Id,
				MessageId: *messageToEdit.MessageId,
			})
			if err != nil {
				log.Printf("Cannot delete message: %s", err)
			}
		}

		parsedUrl, err := url.Parse(messageText)
		if err != nil {
			log.Printf("Cannot parse url: %s", err)
			SendBadUrlMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		savedVideo, err := dbClient.GetVideo(messageText)
		if err != nil {
			log.Printf("Cannot get video from db: %s", err)
		}
		if savedVideo != nil {
			_, err := tgClient.SendVideo(&telegram.SendVideo{
				ChatId:           chat.Id,
				Video:            savedVideo.FileId,
				ReplyToMessageId: message.MessageId,
				Caption:          &messageText,
			})
			if err != nil {
				log.Printf("Cannot send video: %s", err)
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
			log.Printf("Cannot parse platform from url: %s", messageText)
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
			log.Printf("Unrechable code: %+v", parsedPlatform)
			w.WriteHeader(http.StatusOK)
			return
		}

		targetUrl, err := resolver.ResolveUrl(parsedUrl.String())
		if err != nil {
			log.Printf("Cannot resolve url: %s", err)
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		res, err := mp4.Fetch(&httpClient, *targetUrl)
		if err != nil {
			log.Printf("Cannot parse target url: %s", err)
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
			log.Printf("Cannot send video: %s", err)
			SendCannotProcessVideoMessage()
			w.WriteHeader(http.StatusOK)
			return
		}

		DeleteMessage()

		if video != nil && video.Video != nil {
			_, err = dbClient.InsertVideo(messageText, video.Video.FileId)
			if err != nil {
				log.Printf("Cannot insert video to db: %s", err)
			}
		}

		w.WriteHeader(http.StatusOK)
	})

	log.Printf("Starting server at %s", listenAddress)
	log.Fatal(http.ListenAndServe(listenAddress, nil))
}
