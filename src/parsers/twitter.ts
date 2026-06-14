import { HttpClient, HttpClientResponse } from "@effect/platform";
import {
  Array as A,
  Context,
  Effect,
  Layer,
  Option,
  Order,
  Schedule,
  Schema,
} from "effect";

class TwitterError extends Schema.TaggedError<TwitterError>()("TwitterError", {
  message: Schema.String,
}) { }

const GuestTokenResponse = Schema.Struct({
  guest_token: Schema.String,
});

const VideoInfoVariant = Schema.Struct({
  bitrate: Schema.OptionFromUndefinedOr(Schema.Int),
  content_type: Schema.String,
  url: Schema.String,
});
type VideoInfoVariant = typeof VideoInfoVariant.Type;
type VideoInfoVariantWithBitrate = VideoInfoVariant & {
  bitrate: Option.Some<number>;
};

const TwitterResponse = Schema.Struct({
  data: Schema.Struct({
    tweetResult: Schema.Struct({
      result: Schema.Struct({
        legacy: Schema.Struct({
          entities: Schema.Struct({
            media: Schema.Array(
              Schema.Struct({
                type: Schema.String,
                video_info: Schema.Struct({
                  variants: Schema.Array(VideoInfoVariant),
                }),
              }),
            ),
          }),
        }),
      }),
    }),
  }),
});

const isMp4Variant = (v: VideoInfoVariant): v is VideoInfoVariantWithBitrate =>
  v.content_type === "video/mp4" && Option.isSome(v.bitrate);

const VideoInfoVariantByBitrateOrder = Order.mapInput(
  Order.number,
  (v: VideoInfoVariantWithBitrate) => v.bitrate.value,
);

const commonHeaders = {
  "User-Agent":
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0",
  Authorization:
    "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA",
  "x-twitter-client-language": "en",
  "x-twitter-active-user": "yes",
  "accept-language": "en",
};

const guestTokenRetryPolicy = Schedule.exponential("50 millis").pipe(
  Schedule.jittered,
  Schedule.intersect(Schedule.recurs(4)),
);
const twitterRetryPolicy = Schedule.exponential("50 millis").pipe(
  Schedule.jittered,
  Schedule.intersect(Schedule.recurs(4)),
);

class TwitterGuestTokenCached extends Context.Tag("TwitterGuestTokenCached")<
  TwitterGuestTokenCached,
  {
    readonly get: () => Effect.Effect<string, TwitterError>;
  }
>() { }

export const TwitterGuestTokenCachedLive = Layer.effect(
  TwitterGuestTokenCached,
  Effect.gen(function*() {
    const cached = yield* Effect.cachedWithTTL(
      Effect.gen(function*() {
        const client = yield* HttpClient.HttpClient;
        const res = yield* client
          .post("https://api.x.com/1.1/guest/activate.json", {
            headers: commonHeaders,
          })
          .pipe(Effect.retry(guestTokenRetryPolicy));
        const body =
          yield* HttpClientResponse.schemaBodyJson(GuestTokenResponse)(res);
        return body.guest_token;
      }).pipe(
        Effect.tapErrorCause(Effect.logError),
        Effect.catchAll(
          (_) => new TwitterError({ message: `Cannot get guest token` }),
        ),
      ),
      "5 minutes",
    );
    return { get: () => cached };
  }),
);

export const getVideoUrl = (sourceUrl: URL) =>
  Effect.gen(function*() {
    const tokenCached = yield* TwitterGuestTokenCached;
    const token = yield* tokenCached.get();
    const tweetId = yield* A.last(sourceUrl.pathname.split("/")).pipe(
      Effect.mapError(
        (_) => new TwitterError({ message: "Cannot find tweet id" }),
      ),
    );
    const url = new URL(
      "https://x.com/i/api/graphql/2ICDjqPd81tulZcYrtpTuQ/TweetResultByRestId",
    );
    url.searchParams.set(
      "variables",
      JSON.stringify({
        tweetId,
        includePromotedContent: false,
        withVoice: false,
        withCommunity: false,
      }),
    );
    const client = yield* HttpClient.HttpClient;
    const res = yield* client
      .get(url, {
        headers: {
          ...commonHeaders,
          "x-guest-token": token,
        },
      })
      .pipe(
        Effect.retry(twitterRetryPolicy),
        Effect.tapErrorCause(Effect.logError),
        Effect.catchTags({
          RequestError: () =>
            new TwitterError({ message: "Cannot make request" }),
          ResponseError: () =>
            new TwitterError({ message: "Twitter response error" }),
        }),
      );
    const data = yield* HttpClientResponse.schemaBodyJson(TwitterResponse)(
      res,
    ).pipe(
      Effect.tapErrorCause(Effect.logError),
      Effect.mapError(
        (_) => new TwitterError({ message: "Cannot parse response" }),
      ),
    );
    const video = yield* A.findFirst(
      data.data.tweetResult.result.legacy.entities.media,
      (v) => v.type === "video",
    ).pipe(
      Effect.mapError(
        (_) => new TwitterError({ message: "Cannot find media of type video" }),
      ),
      Effect.map((v) => A.filter(v.video_info.variants, isMp4Variant)),
      Effect.flatMap((v) =>
        A.match(v, {
          onEmpty: () =>
            Effect.fail(new TwitterError({ message: "Cannot find video" })),
          onNonEmpty: Effect.succeed,
        }),
      ),
      Effect.map((v) => A.max(v, VideoInfoVariantByBitrateOrder)),
    );
    return video.url;
  });
