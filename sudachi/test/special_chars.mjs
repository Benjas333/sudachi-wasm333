import { promises } from 'fs';
import { SudachiStateless, TokenizeMode } from '../pkg/sudachi.js';

console.log("initializing...");

const sudachi = new SudachiStateless();

console.time('initialized');
// const dic_path = '../resources/system_20210802.dic';
const dic_path = '../resources/system_latest.dic';

const dic = await promises.readFile(dic_path);
const dic_bytes = new Uint8Array(dic);
await sudachi.initialize_from_bytes(dic_bytes);
// await sudachi.initialize(dic_path, promises.readFile);
console.timeEnd('initialized');

const special_chars = [
  "（", "）",
  "｛", "｝",
  "［", "］",
  "【", "】",
  "、", "，",
  "゠", "＝",
  "…", "‥",
  "。",
  "〽",
  "「", "」",
  "『", "』",
  "〝", "〟",
  "　",
  "〜",
  "：",
  "！",
  "？",
  "♪",
];
const successful_chars = [];
const failed_chars = [];
for (const char of special_chars) {
  console.log(`Trying to tokenize: ${char}`);
  try {
    console.log(sudachi.tokenize_raw(char, TokenizeMode.B));
  } catch (error) {
    failed_chars.push(char);
    console.error(error);
    continue;
  }
  successful_chars.push(char);
}

console.log("Successful chars:", successful_chars.join(' '));
console.log("Failed chars:", failed_chars.join(' '));
