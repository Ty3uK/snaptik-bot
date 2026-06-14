import {
  FetchHttpClient,
  HttpClient,
  HttpClientRequest,
  HttpLayerRouter,
  HttpServerRequest,
  HttpServerResponse,
} from "@effect/platform";
import { BunHttpServer, BunRuntime } from "@effect/platform-bun";
import { Effect, Layer, Option } from "effect";
import { FormDataBuilder } from "./formData";
import { Update } from "./model";
import { getVideoUrl, TwitterGuestTokenCachedLive } from "./parsers/twitter";
import { parseEntities } from "./utils";

const HealthRoute = HttpLayerRouter.use(
  Effect.fn(function*(router) {
    yield* router.add("GET", "/health", HttpServerResponse.text("OK"));
  }),
);

const TelegramUpdateRoute = HttpLayerRouter.use(
  Effect.fn(function*(router) {
    yield* router.add(
      "POST",
      "/update",
      Effect.gen(function*() {
        const update = yield* HttpServerRequest.schemaBodyJson(Update);
        if (
          Option.isSome(update.message) &&
          Option.isSome(update.message.value.text) &&
          Option.isSome(update.message.value.entities) &&
          Option.isSome(update.message.value.chat)
        ) {
          const entities = parseEntities(
            update.message.value.text.value,
            update.message.value.entities.value,
          );
          const url = yield* getVideoUrl(new URL(entities[0]?.text ?? ""));
          const client = yield* HttpClient.HttpClient;
          const video = yield* client
            .get(url)
            .pipe(Effect.tapError(Effect.logError));
          const fd = new FormDataBuilder()
            .addTextField(
              "chat_id",
              update.message.value.chat.value.id.toString(),
            )
            .addStreamField("video", "video.mp4", video.stream, {
              contentType: "video/mp4",
            })
            .build();
          const req = HttpClientRequest.post(`${Bun.env.API_URL}/sendVideo`).pipe(
            HttpClientRequest.bodyStream(fd.stream, {
              contentType: `multipart/form-data; boundary=${fd.boundary}`,
            }),
          );
          const res = yield* client
            .execute(req)
            .pipe(Effect.tapError(Effect.logError));
          const json = yield* res.json;
          yield* Effect.logInfo(JSON.stringify(json, null, 2));
        }
        return HttpServerResponse.empty();
      }).pipe(
        Effect.tapErrorCause(Effect.logError),
        Effect.orElseSucceed(HttpServerResponse.empty),
      ),
    );
  }),
);

const AllRoutes = Layer.mergeAll(HealthRoute, TelegramUpdateRoute);

HttpLayerRouter.serve(AllRoutes).pipe(
  Layer.provide(TwitterGuestTokenCachedLive),
  Layer.provide(FetchHttpClient.layer),
  Layer.provide(BunHttpServer.layer({ port: 3000 })),
  Layer.launch,
  BunRuntime.runMain,
);
