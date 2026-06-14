import { Chunk, Stream } from "effect";

export class FormDataBuilder<E = never> {
  constructor(
    private chunk: Chunk.Chunk<Stream.Stream<Uint8Array, E>> = Chunk.empty(),
    private encoder: TextEncoder = new TextEncoder(),
    private boundary = "----my-boundary",
  ) { }

  addTextField(name: string, value: string) {
    this.chunk = this.chunk.pipe(
      Chunk.append(
        Stream.succeed(
          this.encoder.encode(
            `--${this.boundary}\r\n` +
            `Content-Disposition: form-data; name="${name}"\r\n\r\n` +
            `${value}\r\n`,
          ),
        ),
      ),
    );
    return this;
  }

  addStreamField<E2>(
    name: string,
    filename: string,
    value: Stream.Stream<Uint8Array, E2>,
    options?: {
      contentType?: string;
    },
  ) {
    const chunk = this.chunk.pipe(
      Chunk.append(
        Stream.succeed(
          this.encoder.encode(
            `--${this.boundary}\r\n` +
            `Content-Disposition: form-data; name="${name}"; filename="${filename}"\r\n` +
            (options?.contentType
              ? `Content-Type: ${options?.contentType}\r\n`
              : ``) +
            `\r\n`,
          ),
        ),
      ),
      Chunk.append(value as Stream.Stream<Uint8Array, E | E2>),
    );
    return new FormDataBuilder(chunk, this.encoder, this.boundary);
  }

  build() {
    return {
      stream: Stream.concatAll(
        this.chunk.pipe(
          Chunk.append(
            Stream.succeed(this.encoder.encode(`\r\n--${this.boundary}--\r\n`)),
          ),
        ),
      ),
      boundary: this.boundary,
    };
  }
}
