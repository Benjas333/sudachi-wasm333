# sudachi-wasm333

> Updated WebAssembly distribution of [sudachi.rs].

This distribution supports both of browser and Node.js.

<!-- #### ✨ [Demo](https://sudachi-wasm.s3.amazonaws.com/index.html) -->

## Features

- Updated structure of the original [sudachi-wasm] to reassemble the actual structure of [sudachi.rs].
- SudachiStateless and SudachiStateful classes implementation.
- Slightly improved library docstrings and types.
- Added dynamic dict loading, so a custom dict path/url can be provided.
- Improved performance.
- Improved file size because of dynamic dict loading.
- Structure kinda inspired in [Kuroshiro] initialization.

## Usage

### Browser

<!-- [v0.1.4.js](https://sudachi-wasm.s3.amazonaws.com/v0.1.4.js) -->

```html
<script type="module">
  if ("serviceWorker" in navigator) {
    await navigator.serviceWorker.register("serviceWorker.js");
  }

  const console = document.querySelector("#console");
  // Please replace to self-hosted script path.
  const { SudachiStateless, TokenizeMode } = await import("/v1.0.0.js");

  const sudachi = new SudachiStateless();

  await sudachi.initialize_browser();

  console.innerText = JSON.stringify(
    JSON.parse(sudachi.tokenize_stringified("今日は良い天気なり。", TokenizeMode.C)),
    null,
    2
  );
</script>
```

#### ⚠ Script is too large

Gzipped script file is also larger than 50 MB 🐘.
Please use the following mechanisms to delivery it.

- [gzip encoding](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Encoding) for compressing
- [Service Worker](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API/Using_Service_Workers) for caching

### Node.js

```bash
npm i sudachi-wasm333
```

Then,

```js
import { promises } from 'fs';
import { SudachiStateless, TokenizeMode } from "sudachi";

const sudachi = new SudachiStateless();

await sudachi.initialize_node(promises.readFile);

console.log(sudachi.tokenize_raw("今日は良い天気なり。", TokenizeMode.C));
```

## Development requirements

- [wasm-pack](https://github.com/rustwasm/wasm-pack)
- [zx](https://github.com/google/zx)

## Build

```bash
wasm-pack build --dev --target web && cd pkg && zx ../wasm-pack-inline.mjs && cd ..
```

## Test

### Browser

```bash
npx http-server
```

Then, access to the [local server](http://127.0.0.1:8080/test/browser.html).

> I actually prefer [Live Server](https://marketplace.visualstudio.com/items?itemName=ritwickdey.LiveServer).<br>
> -Benjas333

### Node.js

```bash
node test/node.mjs
```

```bash
node test/node_stateful.mjs
```

```bash
node test/special_chars.mjs
```

## TODO

- Add public link (like the original sudachi-wasm: https://sudachi-wasm.s3.amazonaws.com/v0.1.4.js).
- Add SudachiStateful examples.
- Improve documentation.
- Edit [README.ja.md](README.ja.md).
- Add demo (like the original sudachi-wasm: https://sudachi-wasm.s3.amazonaws.com/index.html).

[sudachi.rs]: https://github.com/WorksApplications/sudachi.rs
[sudachi-wasm]: https://github.com/hata6502/sudachi-wasm
[Kuroshiro]: https://github.com/hexenq/kuroshiro
