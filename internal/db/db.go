package db

import (
	"database/sql"
	"fmt"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

type DbClient struct {
	db *sql.DB
}

type Video struct {
	Url       string    `json:"url"`
	FileId    string    `json:"file_id"`
	CreatedAt time.Time `json:"created_at"`
}

func NewDbClient(dbPath string) (*DbClient, error) {
	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		return nil, fmt.Errorf("Cannot open db: %e", err)
	}

	_, err = db.Exec("CREATE TABLE IF NOT EXISTS videos (url TEXT PRIMARY KEY, file_id TEXT NOT NULL, created_at DATETIME NOT NULL)")
	if err != nil {
		return nil, fmt.Errorf("Cannot create table: %s", err)
	}

	return &DbClient{
		db: db,
	}, nil
}

func (client *DbClient) Close() error {
	return client.db.Close()
}

func (client *DbClient) GetVideo(url string) (*Video, error) {
	var video Video
	err := client.db.QueryRow("SELECT url, file_id FROM videos WHERE url=?", url).Scan(&video.Url, &video.FileId)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, nil
		}

		return nil, fmt.Errorf("Cannot query row: %s", err)
	}
	return &video, nil
}

func (client *DbClient) InsertVideo(url string, fileId string) (bool, error) {
	stmt, err := client.db.Prepare("INSERT INTO videos VALUES (?, ?, ?)")
	if err != nil {
		return false, fmt.Errorf("Cannot prepare db statement: %e", err)
	}
	defer stmt.Close()

	_, err = stmt.Exec(url, fileId, time.Now())
	if err != nil {
		return false, fmt.Errorf("Cannot exec db statement: %e", err)
	}

	return true, nil
}
