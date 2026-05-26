import {
  FetchHttpClient,
  HttpLayerRouter,
  HttpServerRequest,
  HttpServerResponse,
} from "@effect/platform";
import { BunHttpServer, BunRuntime } from "@effect/platform-bun";
import { Effect, Layer, Option } from "effect";
import { TgUpdate } from "./model";
import { getVideoUrl } from "./parsers/twitter";
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
        const req = yield* HttpServerRequest.schemaBodyJson(TgUpdate);
        if (
          Option.isSome(req.message) &&
          Option.isSome(req.message.value.text) &&
          Option.isSome(req.message.value.entities)
        ) {
          const entities = parseEntities(
            req.message.value.text.value,
            req.message.value.entities.value,
          );
          const url = yield* getVideoUrl(new URL(entities[0]?.text ?? ""));
          yield* Effect.logInfo(url);
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
  Layer.provide(FetchHttpClient.layer),
  Layer.provide(BunHttpServer.layer({ port: 3000 })),
  Layer.launch,
  BunRuntime.runMain,
);
