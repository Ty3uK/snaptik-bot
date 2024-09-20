package telegram

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"mime/multipart"
	"net/http"
	"strconv"
	"sync"

	"go.uber.org/zap"
)

type ChatType string

const (
	ChatTypePrivate    ChatType = "private"
	ChatTypeGroup      ChatType = "private"
	ChatTypeSupergroup ChatType = "private"
	ChatTypeChannel    ChatType = "private"
)

type SetWebhook struct {
	Url         string  `json:"url"`
	SecretToken *string `json:"secret_token,omitempty"`
}

type Chat struct {
	Id       int      `json:"id"`
	ChatType ChatType `json:"type"`
}

type Video struct {
	FileId string `json:"file_id"`
}

type User struct {
	LanguageCode string `json:"language_code"`
	IsBot        bool   `json:"is_bot"`
	IsPremium    bool   `json:"is_premium"`
}

type Message struct {
	MessageId      *int     `json:"message_id,omitempty"`
	Chat           *Chat    `json:"chat,omitempty"`
	Text           *string  `json:"text,omitempty"`
	ReplyToMessage *Message `json:"reply_to_message,omitempty"`
	Video          *Video   `json:"video,omitempty"`
	From           *User    `json:"from,omitempty"`
}

type Update struct {
	UpdateId int      `json:"update_id"`
	Message  *Message `json:"message,omitempty"`
}

type LinkPreviewOptions struct {
	IsDisabled bool `json:"is_disabled"`
}

type SendMessage struct {
	ChatId             int                 `json:"chat_id"`
	Text               string              `json:"text"`
	ReplyToMessageId   *int                `json:"reply_to_message_id,omitempty"`
	LinkPreviewOptions *LinkPreviewOptions `json:"link_preview_options,omitempty"`
}

type SendVideo struct {
	ChatId           int     `json:"chat_id"`
	Video            string  `json:"video"`
	ReplyToMessageId *int    `json:"reply_to_message_id"`
	Caption          *string `json:"caption"`
}

type SendVideoFile struct {
	ChatId           int
	Video            *io.ReadCloser
	VideoMeta        *[]byte
	ReplyToMessageId int
	Caption          string
	Width            uint64
	Height           uint64
}

type EditMessageText struct {
	ChatId    int    `json:"chat_id"`
	MessageId int    `json:"message_id"`
	Text      string `json:"text"`
}

type DeleteMessage struct {
	ChatId    int `json:"chat_id"`
	MessageId int `json:"message_id"`
}

type Response[T any] struct {
	Ok          bool    `json:"ok"`
	Description *string `json:"description,omitempty"`
	ErrorCode   *int    `json:"error_code,omitempty"`
	Result      *T      `json:"result,omitempty"`
}

type TelegramClient struct {
	apiPath    string
	httpClient *http.Client
	logger     *zap.SugaredLogger
}

func NewTelegramClient(botToken string, httpClient *http.Client, logger *zap.SugaredLogger) TelegramClient {
	return TelegramClient{
		apiPath:    fmt.Sprintf("https://api.telegram.org/bot%s", botToken),
		httpClient: httpClient,
		logger:     logger,
	}
}

func (client *TelegramClient) SetWebook(webhook *SetWebhook) (*bool, error) {
	return sendRequest[SetWebhook, bool](client, "setWebhook", webhook)
}

func (client *TelegramClient) SendMessage(message *SendMessage) (*Message, error) {
	return sendRequest[SendMessage, Message](client, "sendMessage", message)
}

func (client *TelegramClient) EditMessageText(editMessageText *EditMessageText) (*Message, error) {
	return sendRequest[EditMessageText, Message](client, "editMessageText", editMessageText)
}

func (client *TelegramClient) DeleteMessage(deleteMessage *DeleteMessage) (*bool, error) {
	return sendRequest[DeleteMessage, bool](client, "deleteMessage", deleteMessage)
}

func (client *TelegramClient) SendVideo(sendVideo *SendVideo) (*Message, error) {
	return sendRequest[SendVideo, Message](client, "sendVideo", sendVideo)
}

func (client *TelegramClient) SendVideoFile(video *SendVideoFile) (*Message, error) {
	buffer := bufferPool.Get().(*[]byte)
	defer bufferPool.Put(buffer)

	r, w := io.Pipe()
	form := multipart.NewWriter(w)

	go func() {
		defer w.Close()
		defer form.Close()
		defer (*video.Video).Close()

		_ = form.WriteField("chat_id", strconv.FormatInt(int64(video.ChatId), 10))
		_ = form.WriteField("reply_to_message_id", strconv.FormatInt(int64(video.ReplyToMessageId), 10))
		_ = form.WriteField("caption", video.Caption)
		_ = form.WriteField("width", strconv.FormatUint(video.Width, 10))
		_ = form.WriteField("height", strconv.FormatUint(video.Height, 10))

		file, err := form.CreateFormFile("video", "video.mp4")
		if err != nil {
			client.logger.Errorf("Cannot create file field: %s", err)
			return
		}

		_, err = file.Write(*video.VideoMeta)
		if err != nil {
			client.logger.Errorf("Cannot write video meta: %s", err)
			return
		}

		_, err = io.CopyBuffer(file, *video.Video, *buffer)
		if err != nil {
			w.CloseWithError(err)
			client.logger.Errorf("Cannot write video: %s", err)
			return
		}
	}()

	req, err := http.NewRequest("POST", fmt.Sprintf("%s/sendVideo", client.apiPath), r)
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %e", err)
	}

	req.Header.Set("Content-Type", form.FormDataContentType())

	res, err := client.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %e", err)
	}

	var response Response[Message]
	defer res.Body.Close()
	err = json.NewDecoder(res.Body).Decode(&response)
	if err != nil {
		return nil, fmt.Errorf("Cannot decode response json: %e", err)
	}

	if !response.Ok {
		return nil, fmt.Errorf("Bad response from sendVideo: %+v", response)
	}

	return response.Result, nil
}

func sendRequest[I any, O any](client *TelegramClient, method string, request *I) (*O, error) {
	body, err := json.Marshal(request)
	if err != nil {
		return nil, fmt.Errorf("Cannot marshal json: %e", err)
	}

	req, err := http.NewRequest("POST", fmt.Sprintf("%s/%s", client.apiPath, method), bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("Cannot create request: %e", err)
	}

	req.Header.Set("Content-Type", "application/json")

	res, err := client.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("Cannot make request: %e", err)
	}

	var response Response[O]
	defer res.Body.Close()
	err = json.NewDecoder(res.Body).Decode(&response)
	if err != nil {
		return nil, fmt.Errorf("Cannot decode response json: %e", err)
	}

	if !response.Ok {
		err, _ := json.Marshal(response)
		return nil, fmt.Errorf("Bad response from %s: %s", method, err)
	}

	return response.Result, nil
}

var bufferPool = sync.Pool{
	New: func() any {
		buffer := make([]byte, 32*1024)
		return &buffer
	},
}
