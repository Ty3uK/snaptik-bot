import { Schema } from "effect";

export const UserId = Schema.Int.pipe(Schema.brand("UserId"));
export const User = Schema.Struct({
  id: UserId,
});

export const ChatId = Schema.Int.pipe(Schema.brand("ChatId"));
export const Chat = Schema.Struct({
  id: ChatId,
});

export const MessageEntity = Schema.Struct({
  type: Schema.String,
  offset: Schema.Int,
  length: Schema.Int,
});

export const MessageId = Schema.Int.pipe(Schema.brand("MessageId"));
export const Message = Schema.Struct({
  message_id: MessageId,
  from: Schema.OptionFromUndefinedOr(User),
  chat: Schema.OptionFromUndefinedOr(Chat),
  text: Schema.OptionFromUndefinedOr(Schema.String),
  entities: Schema.OptionFromUndefinedOr(Schema.Array(MessageEntity)),
});

export const UpdateId = Schema.Int.pipe(Schema.brand("UpdateId"));
export const Update = Schema.Struct({
  update_id: UpdateId,
  message: Schema.OptionFromUndefinedOr(Message),
});

export const Response = <A, I, R>(item: Schema.Schema<A, I, R>) =>
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
  );
