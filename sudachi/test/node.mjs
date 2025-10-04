
console.log("importing...");

import { promises, readFileSync } from 'fs';
import { SudachiStateless, TokenizeMode } from '../pkg/sudachi.js';
// import { initializeSudachi, tokenize, TokenizeMode } from "../pkg/sudachi.js";

console.time("Stateless")
console.log("initializing...");

const sudachi = new SudachiStateless();

console.time('initialized')
const dict_path = '../resources/system_latest.dic'
await sudachi.initialize_node(promises.readFile);
console.timeEnd('initialized')
// await initializeSudachi();

console.log("tokenizing 1...");
console.log(sudachi.tokenize_stringified("今日は良い天気なり。", TokenizeMode.C));

console.log("tokenizing 2...");
console.log(sudachi.tokenize_stringified("明日は悪い予報なり。", TokenizeMode.B));

console.log("tokenizing 3...");
console.log(sudachi.tokenize_stringified("共鳴する 運命の波紋 胸に", TokenizeMode.A));

const long_text = `
  親譲りの無鉄砲で小供の時から損ばかりしている。
  小学校に居る時分学校の二階から飛び降りて一週間ほど腰を抜かした事がある。
  なぜそんな無闇をしたと聞く人があるかも知れぬ。
  別段深い理由でもない。
  新築の二階から首を出していたら、同級生の一人が冗談に、いくら威張っても、そこから飛び降りる事は出来まい。
  弱虫やーい。
  と囃したからである。
  小使に負ぶさって帰って来た時、おやじが大きな眼をして二階ぐらいから飛び降りて腰を抜かす奴があるかと云ったから、この次は抜かさずに飛んで見せますと答え
  た。
  （青空文庫より）
`;
const lines = long_text.split('\n').filter(line => line.trim());
for (let i = 0; i < lines.length; i++) {
  const line = lines[i];
  console.log(`Tokenizing line ${i + 1}...`);
  console.log(sudachi.tokenize_stringified(line.trim(), TokenizeMode.A));
}
console.log("tokenizing 4...");
console.log(
  sudachi.tokenize_stringified(
    long_text,
    TokenizeMode.C
  )
);
console.timeEnd("Stateless")
