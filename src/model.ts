import { Schema } from "effect";

export const TgUser = Schema.Struct({
  id: Schema.Int,
}).pipe(Schema.brand("TgUser"));

export const TgChat = Schema.Struct({
  id: Schema.Int,
}).pipe(Schema.brand("TgChat"));

export const TgMessageEntity = Schema.Struct({
  type: Schema.String,
  offset: Schema.Int,
  length: Schema.Int,
}).pipe(Schema.brand("TgMessageEntity"));

export const TgMessage = Schema.Struct({
  message_id: Schema.Int,
  from: Schema.OptionFromUndefinedOr(TgUser),
  chat: Schema.OptionFromUndefinedOr(TgChat),
  text: Schema.OptionFromUndefinedOr(Schema.String),
  entities: Schema.OptionFromUndefinedOr(Schema.Array(TgMessageEntity)),
}).pipe(Schema.brand("TgMessage"));

export const TgResponse = <A, I, R>(item: Schema.Schema<A, I, R>) =>
  Schema.Union(
    Schema.Struct({
      ok: Schema.Literal(true),
      result: item,
    }),
    Schema.Struct({
      ok: Schema.Literal(false),
      error_code: Schema.Int,
      description: Schema.String,
    }),
  ).pipe(Schema.brand("TgResponse"));

export const TgUpdate = Schema.Struct({
  update_id: Schema.Int,
  message: Schema.OptionFromUndefinedOr(TgMessage),
}).pipe(Schema.brand("TgUpdate"));
