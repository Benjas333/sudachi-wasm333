#!/usr/bin/env zx
console.log("Parsing built package...");

const packageObject = JSON.parse(await fs.readFile('pkg/package.json', 'utf-8'));
const wasmPath = packageObject.files.find(file => file.endsWith('.wasm'));
const wasm = await fs.readFile(`pkg/${wasmPath}`);
const wasmBASE64 = wasm.toString('base64');

const initializeScript = `
function get_default_dic_path() {
    const path = 'resources/system.dic';
    const dir = import.meta.dirname;
    if (typeof dir !== 'undefined') {
        return \`\${dir}/\${path}\`;
    }
    const url = import.meta.url;
    if (!url.startsWith('file:///')) {
        return (new URL(path, url)).toString();
    }
    throw new Error('Unsupported platform');
}

const wasmBASE64 = '${wasmBASE64}';

let bytes;

if (typeof atob === 'function') {
    const binary = atob(wasmBASE64);
    bytes = new Uint8Array(binary.length);
    
    for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
    }  
} else if (typeof Buffer === 'function') {
    bytes = Buffer.from(wasmBASE64, 'base64');
} else {
    throw new Error('Unsupported platform');
}

initSync({ module: bytes });
`

if (!packageObject.module) {
    packageObject.module = "sudachi.js";
}

await fs.appendFile(`pkg/${packageObject.module}`, initializeScript);

packageObject.files = packageObject.files.filter(file => !file.endsWith('.wasm'));
packageObject.files.push("resources");
packageObject.main = packageObject.module;
packageObject.name = "sudachi-wasm333"
packageObject.type = "module";

await fs.writeFile('pkg/package.json', JSON.stringify(packageObject, null, 2));
console.log("✅ Built package parsed and modified successfully.");
