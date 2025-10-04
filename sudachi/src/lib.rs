/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Clone of [Sudachi](https://github.com/WorksApplications/Sudachi),
//! a Japanese morphological analyzer
//!
//! There is no public API for the initial release.
//! Issue: https://github.com/WorksApplications/sudachi.rs/issues/28
//!
//! Also, there are to mostly
//! [SudachiPy-compatible Python bindings](https://worksapplications.github.io/sudachi.rs/python/).

pub mod analysis;
pub mod config;
pub mod dic;
pub mod error;
pub mod input_text;
pub mod plugin;
pub mod sentence_detector;
pub mod sentence_splitter;
pub(crate) mod util;

mod hash;
pub mod pos;
#[cfg(test)]
pub mod test;

pub mod prelude {
    pub use crate::{
        analysis::mlist::MorphemeList, analysis::morpheme::Morpheme, analysis::Mode,
        error::SudachiError, error::SudachiResult,
    };
}

extern crate wasm_bindgen;

use std::sync::Arc;

use config::Config;
use analysis::Mode;
use serde::{Serialize, Deserialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use web_sys::{js_sys, ReadableStreamDefaultReader, Request, RequestInit, Response};
use wasm_bindgen_futures::js_sys::{Function, Uint8Array, Promise};

use crate::analysis::stateful_tokenizer::StatefulTokenizer;
use crate::analysis::stateless_tokenizer::StatelessTokenizer;
use crate::analysis::Tokenize;
use crate::dic::dictionary::JapaneseDictionary;
use crate::dic::storage::{Storage, SudachiDicData};
use crate::prelude::{Morpheme, MorphemeList};

#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    #[cfg(feature = "console_log")]
    console_log::init_with_level(log::Level::Info).expect("Error initializing log");
}

#[wasm_bindgen]
extern "C" {
    fn get_default_dic_path() -> String;
}

/// Unit to split text
///
/// Some examples:
/// ```text
/// A：選挙/管理/委員/会
/// B：選挙/管理/委員会
/// C：選挙管理委員会
///
/// A：客室/乗務/員
/// B：客室/乗務員
/// C：客室乗務員
///
/// A：労働/者/協同/組合
/// B：労働者/協同/組合
/// C：労働者協同組合
///
/// A：機能/性/食品
/// B：機能性/食品
/// C：機能性食品
/// ```
///
/// See [Sudachi documentation](https://github.com/WorksApplications/Sudachi#the-modes-of-splitting)
/// for more details
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenizeMode {
    /// Short
    A,

    /// Middle (similar to "word")
    B,

    /// Named Entity
    C,
}

impl From<TokenizeMode> for Mode {
    fn from(mode: TokenizeMode) -> Self {
        match mode {
            TokenizeMode::A => Mode::A,
            TokenizeMode::B => Mode::B,
            TokenizeMode::C => Mode::C,
        }
    }
}

impl From<Mode> for TokenizeMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::A => TokenizeMode::A,
            Mode::B => TokenizeMode::B,
            Mode::C => TokenizeMode::C,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct TokenMorpheme {
    pub surface: String,
    pub poses: Vec<String>,
    pub normalized_form: String,
    pub reading_form: String,
    pub dictionary_form: String,
    pub word_id: i32,
    pub oov: bool,
    pub begin: usize,
    pub end: usize,
}

impl<'a, D: analysis::stateless_tokenizer::DictionaryAccess> From<Morpheme<'a, D>> for TokenMorpheme {
    fn from(morpheme: Morpheme<'a, D>) -> Self {
        Self {
            surface: morpheme.surface().to_string(),
            poses: morpheme.part_of_speech().to_vec(),
            normalized_form: morpheme.normalized_form().to_string(),
            reading_form: morpheme.reading_form().to_string(),
            dictionary_form: morpheme.dictionary_form().to_string(),
            word_id: morpheme.word_id().as_raw() as i32,
            oov: morpheme.is_oov(),
            begin: morpheme.begin(),
            end: morpheme.end(),
        }
    }
}

#[derive(Serialize, Debug)]
struct WasmError {
    error: String,
    details: String,
}

impl From<WasmError> for JsValue {
    fn from(err: WasmError) -> JsValue {
        to_value(&err).unwrap_or_else(|_| JsValue::from_str(r#"{"error":"SerializationError","details":"Failed to serialize error"}"#))
    }
}

async fn load_dict_from_path(
    dict_path: Option<String>,
    read_file_func: Function,
) -> Result<Vec<u8>, JsValue> {
    let url = dict_path.unwrap_or_else(get_default_dic_path);
    let this = JsValue::NULL;
    let url_js = JsValue::from_str(&url);
    let mut result = read_file_func.call1(&this, &url_js)
        .map_err(|e| JsValue::from(WasmError {
            error: "FileReadError".to_string(),
            details: format!("Failed to call read_file_func function: {:?}", e),
        }))?;

    if result.is_instance_of::<Promise>() {
        let promise: Promise = result.dyn_into()
            .map_err(|e| JsValue::from(WasmError {
                error: "FileReadError".to_string(),
                details: format!("Expected a Promise from read_file_func, got: {:?}", e),
            }))?;

        result = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| JsValue::from(WasmError {
                error: "FileReadError".to_string(),
                details: format!("Failed to resolve read_file_func promise: {:?}", e),
            }))?;
    }

    if !(result.is_instance_of::<Uint8Array>()) {
        return Err(JsValue::from(WasmError {
            error: "FileReadError".to_string(),
            details: format!("Expected Uint8Array from read_file_func, got: {:?}", result),
        }));
    }
    Ok(Uint8Array::from(result).to_vec())
}

async fn load_dict_from_url(
    dict_url: Option<String>,
) -> Result<Vec<u8>, JsValue> {
    let path = dict_url.unwrap_or_else(get_default_dic_path);
    // log::info!("Trying to load dict from: {}", path);

    let opts = RequestInit::new();
    opts.set_method("GET");
    
    let request = Request::new_with_str_and_init(&path, &opts)
        .map_err(|e| JsValue::from(WasmError {
            error: "FetchError".to_string(),
            details: format!("Failed to create fetch request: {:?}", e),
        }))?;
    
    let window = web_sys::window().ok_or_else(|| JsValue::from(WasmError {
        error: "EnvironmentError".to_string(),
        details: "No window object available (are you in a browser?)".to_string(),
    }))?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| JsValue::from(WasmError {
            error: "FetchError".to_string(),
            details: format!("Failed to fetch file: {:?}", e),
        }))?;
    
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into()
        .map_err(|e| JsValue::from(WasmError {
            error: "FetchError".to_string(),
            details: format!("Invalid fetch response: {:?}", e),
        }))?;
    
    if !resp.ok() {
        return Err(JsValue::from(WasmError {
            error: "FetchError".to_string(),
            details: format!("HTTP error: {} {}", resp.status(), resp.status_text())
        }));
    }

    // log::info!("Response OK, status: {}, content-length: {:?}", resp.status(), resp.headers().get("Content-Length"));

    let body = resp.body().ok_or_else(|| JsValue::from(WasmError {
        error: "FetchError".to_string(),
        details: "No body in response".to_string(),
    }))?;
    let reader: ReadableStreamDefaultReader = body.get_reader()
        .dyn_into()
        .map_err(|e| JsValue::from(WasmError {
            error: "StreamError".to_string(),
            details: format!("Failed to cast to ReadableStreamDefaultReader: {:?}", e),
        }))?;

    let content_length: usize = resp.headers().get("Content-Length")
        .and_then(|s| s
            .unwrap_or("0".to_string())
            .parse()
            .map_err(|e| JsValue::from(WasmError {
                error: "ParseError".to_string(),
                details: format!("Failed to parse data: {:?}", e),
            }))
        )
        .unwrap_or(0);
    let mut dict_bytes: Vec<u8> = if content_length > 0 {
        Vec::with_capacity(content_length)
    } else {
        Vec::new()
    };

    loop {
        let read_result = wasm_bindgen_futures::JsFuture::from(reader.read())
            .await
            .map_err(|e| JsValue::from(WasmError {
                error: "StreamError".to_string(),
                details: format!("Failed to read stream chunk: {:?}", e),
            }))?;
        
        let done = js_sys::Reflect::get(&read_result, &JsValue::from_str("done"))
            .map_err(|e| JsValue::from(WasmError {
                error: "StreamError".to_string(),
                details: format!("Failed to get 'done' from chunk: {:?}", e),
            }))?
            .as_bool()
            .unwrap_or(false);

        if done {
            // log::info!("Stream reading complete");
            break;
        }

        let value = js_sys::Reflect::get(&read_result, &JsValue::from_str("value"))
            .map_err(|e| JsValue::from(WasmError {
                error: "StreamError".to_string(),
                details: format!("Failed to get 'value' from chunk: {:?}", e),
            }))?;
        
        let chunk: Uint8Array = value.dyn_into()
            .map_err(|e| JsValue::from(WasmError {
                error: "StreamError".to_string(),
                details: format!("Chunk is not Uint8Array: {:?}", e),
            }))?;
        
        // let chunk_len = chunk.length() as usize;
        // log::info!("Chunk received, size: {}, total so far: {}", chunk_len, dict_bytes.len() + chunk_len);
        dict_bytes.extend_from_slice(&chunk.to_vec());
    }

    // log::info!("Total bytes loaded: {}", dict_bytes.len());
    
    if dict_bytes.is_empty() {
        return Err(JsValue::from(WasmError {
            error: "EmptyResponse".to_string(),
            details: "Received empty data from stream".to_string(),
        }));
    }
    
    Ok(dict_bytes)
}

fn create_config_and_dictionary(dict_bytes: &[u8]) -> Result<(Config, Arc<JapaneseDictionary>), JsValue> {
    if dict_bytes.is_empty() {
        return Err(JsValue::from(WasmError {
            error: "InvalidDictionary".to_string(),
            details: "Dictionary bytes cannot be empty".to_string(),
        }));
    }

    let config = Config::new_embedded()
        .map_err(|e| JsValue::from(WasmError {
            error: "ConfigInitError".to_string(),
            details: format!("Failed to initialize config: {}", e)
        }))?;
    
    let storage = SudachiDicData::new(Storage::Owned(dict_bytes.to_vec()));

    let dictionary = JapaneseDictionary::from_cfg_storage_with_embedded_chardef(&config, storage)
        .map_err(|e| JsValue::from(WasmError {
            error: "DictionaryLoadError".to_string(),
            details: format!("Failed to load dictionary: {}", e),
        }))?;
    
    Ok((config, Arc::new(dictionary)))
}

/// Implementation of a Tokenizer which does not have tokenization state.
///
/// This is a wrapper which is generic over dictionary pointers.
#[wasm_bindgen]
pub struct SudachiStateless {
    tokenizer: Option<StatelessTokenizer<Arc<JapaneseDictionary>>>,
    config: Option<Config>,
}

#[wasm_bindgen]
impl SudachiStateless {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            tokenizer: None,
            config: None,
        }
    }

    /// Initializes the tokenizer with the given dictionary file url. If not given, the default one is used.
    /// 
    /// Returns an error object `{ error: string, details: string }` if initialization fails.
    #[wasm_bindgen]
    pub async fn initialize_browser(&mut self, dict_url: Option<String>) -> Result<(), JsValue> {
        let dict_bytes = load_dict_from_url(dict_url).await?;
        self.initialize_from_bytes(dict_bytes.as_slice())
    }

    /// Initializes the tokenizer with the given dictionary file path. If not given, the default one is used.
    /// 
    /// Uses the provided `read_file_func` function (e.g., fs.readFileSync) to load the file.
    /// 
    /// Returns an error object `{ error: string, details: string }` if initialization fails.
    #[wasm_bindgen]
    pub async fn initialize_node(&mut self, read_file_func: Function, dict_path: Option<String>) -> Result<(), JsValue> {
        let dict_bytes = load_dict_from_path(dict_path, read_file_func).await?;
        self.initialize_from_bytes(dict_bytes.as_slice())
    }

    /// Initializes the tokenizer with the given bytes.
    /// Returns an error object `{ error: string, details: string }` if initialization fails.
    #[wasm_bindgen]
    pub fn initialize_from_bytes(&mut self, dict_bytes: &[u8]) -> Result<(), JsValue> {
        let (config, dictionary) = create_config_and_dictionary(dict_bytes)?;
        let tokenizer = StatelessTokenizer::new(dictionary);

        self.tokenizer = Some(tokenizer);
        self.config = Some(config);

        Ok(())
    }
    
    /// Internal method to tokenize the input string.
    fn _tokenize(&self, input: String, mode: TokenizeMode, enable_debug: Option<bool>) -> Result<Vec<TokenMorpheme>, JsValue> {
        let enable_debug = match enable_debug {
            None => false,
            Some(v) => v,
        };

        // if input.len() > 1_000_000 {
        //     return Err(JsValue::from(WasmError {
        //         error: "InputError".to_string(),
        //         details: format!("Input too long: {} bytes, maximum allowed is 1MB", input.len()),
        //     }));
        // }
        
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Tokenizer not initialized. Call initialize() first.".to_string()
            }))?;
        
        let morphemes = tokenizer
            .tokenize(&input, mode.into(), enable_debug)
            .map_err(|e| JsValue::from(WasmError {
                error: "TokenizationError".to_string(),
                details: format!("Failed to tokenize: {}", e),
            }))?;
    
        let described_morphemes = morphemes
            .iter()
            .map(|m| Ok(TokenMorpheme::from(m)))
            .collect::<Result<Vec<_>, std::string::FromUtf8Error>>()
            .map_err(|e| JsValue::from(WasmError {
                error: "TokenizationError".to_string(),
                details: format!("Failed to process morphemes: {}", e),
            }))?;

        Ok(described_morphemes)
    }

    /// Tokenizes the input string using the specified mode from TokenizeMode.
    /// 
    /// Returns a JSON string of morphemes on success, or an error object `{ error: string, details: string }` on failure.
    #[wasm_bindgen]
    pub fn tokenize_stringified(&self, input: String, mode: TokenizeMode, enable_debug: Option<bool>) -> Result<String, JsValue> {
        let described_morphemes = self._tokenize(input, mode, enable_debug)?;
    
        serde_json::to_string(&described_morphemes).map_err(|e| JsValue::from(WasmError {
            error: "SerializationError".to_string(),
            details: format!("Failed to serialize morphemes: {}", e),
        }))
    }

    /// Tokenizes the input string using the specified mode from TokenizeMode.
    /// 
    /// Returns an array of morpheme objects on success, or an error object `{ error: string, details: string }` on failure.
    #[wasm_bindgen]
    pub fn tokenize_raw(&self, input: String, mode: TokenizeMode, enable_debug: Option<bool>) -> Result<Vec<TokenMorpheme>, JsValue> {
        self._tokenize(input, mode, enable_debug)
    }
    
    /// Resets the tokenizer, uninitializing it.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.tokenizer = None;
        self.config = None;
    }

    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        self.tokenizer.is_some()
    }
}

/// Implementation of a Tokenizer which have tokenization state.
///
/// Useful when no need to define TokenizeMode and/or debug every time.
#[wasm_bindgen]
pub struct SudachiStateful {
    tokenizer: Option<StatefulTokenizer<Arc<JapaneseDictionary>>>,
    config: Option<Config>,
    morpheme_list: Option<MorphemeList<Arc<JapaneseDictionary>>>,
}

#[wasm_bindgen]
impl SudachiStateful {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            tokenizer: None,
            config: None,
            morpheme_list: None,
        }
    }

    /// Initializes the tokenizer with the given dictionary file url. If not given, the default one is used.
    /// 
    /// Returns an error object `{ error: string, details: string }` if initialization fails.
    #[wasm_bindgen]
    pub async fn initialize_browser(&mut self, mode: TokenizeMode, debug: Option<bool>, dict_url: Option<String>) -> Result<(), JsValue> {
        let dict_bytes = load_dict_from_url(dict_url).await?;
        self.initialize_from_bytes(dict_bytes.as_slice(), mode, debug)
    }

    /// Initializes the tokenizer with the given dictionary file path. If not given, the default one is used.
    /// 
    /// Uses the provided `read_file_func` function (e.g., fs.readFileSync) to load the file.
    /// 
    /// Returns an error object `{ error: string, details: string }` if initialization fails.
    #[wasm_bindgen]
    pub async fn initialize_node(&mut self, read_file_func: Function, mode: TokenizeMode, debug: Option<bool>, dict_path: Option<String>) -> Result<(), JsValue> {
        let dict_bytes = load_dict_from_path(dict_path, read_file_func).await?;
        self.initialize_from_bytes(dict_bytes.as_slice(), mode, debug)
    }

    /// Initializes the tokenizer with the given bytes.
    /// Returns an error object `{ error: string, details: string }` if initialization fails.
    #[wasm_bindgen]
    pub fn initialize_from_bytes(&mut self, dict_bytes: &[u8], mode: TokenizeMode, debug: Option<bool>) -> Result<(), JsValue> {
        let (config, dictionary) = create_config_and_dictionary(dict_bytes)?;
        let debug = debug.unwrap_or(false);
        
        let tokenizer = StatefulTokenizer::create(dictionary, debug, mode.into());
        let morpheme_list = MorphemeList::empty(tokenizer.dict_clone());

        self.tokenizer = Some(tokenizer);
        self.config = Some(config);
        self.morpheme_list = Some(morpheme_list);

        Ok(())
    }
    
    /// Internal method to tokenize the input string.
    fn _tokenize(&mut self, input: String, mode: Option<TokenizeMode>) -> Result<Vec<TokenMorpheme>, JsValue> {
        let mode = match mode {
            None => None,
            Some(v) => Some(v.into()),
        };
        // if input.len() > 1_000_000 {
        //     return Err(JsValue::from(WasmError {
        //         error: "InputError".to_string(),
        //         details: format!("Input too long: {} bytes, maximum allowed is 1MB", input.len()),
        //     }));
        // }
        
        let mut tokenizer = self.tokenizer.as_mut()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Tokenizer not initialized. Call initialize() first.".to_string()
            }))?;
        let morpheme_list = self.morpheme_list.as_mut()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Morpheme List not initialized. Call initialize() first.".to_string()
            }))?;
        
        let previous_mode = mode.map(|m| tokenizer.set_mode(m));
        let mut tokenizer = scopeguard::guard(&mut tokenizer, |t| {
            previous_mode.map(|m| t.set_mode(m));
        });
        
        tokenizer.reset().push_str(&input);
        tokenizer
            .do_tokenize()
            .map_err(|e| JsValue::from(WasmError {
                error: "TokenizationError".to_string(),
                details: format!("Failed to tokenize: {}", e),
            }))?;

        morpheme_list.collect_results(&mut tokenizer)
            .map_err(|e| JsValue::from(WasmError {
                error: "TokenizationError".to_string(),
                details: format!("Failed to get morpheme list: {}", e),
            }))?;
    
        let described_morphemes = morpheme_list
            .iter()
            .map(|m| Ok(TokenMorpheme::from(m)))
            .collect::<Result<Vec<_>, std::string::FromUtf8Error>>()
            .map_err(|e| JsValue::from(WasmError {
                error: "TokenizationError".to_string(),
                details: format!("Failed to process morphemes: {}", e),
            }))?;

        Ok(described_morphemes)
    }

    /// Tokenizes the input string using the defined mode from TokenizeMode at initialization.
    /// If a mode is provided, that mode will be used temporarily until the end of the function execution.
    /// 
    /// Returns a JSON string of morphemes on success, or an error object `{ error: string, details: string }` on failure.
    #[wasm_bindgen]
    pub fn tokenize_stringified(&mut self, input: String, mode: Option<TokenizeMode>) -> Result<String, JsValue> {
        let described_morphemes = self._tokenize(input, mode)?;
    
        serde_json::to_string(&described_morphemes).map_err(|e| JsValue::from(WasmError {
            error: "SerializationError".to_string(),
            details: format!("Failed to serialize morphemes: {}", e),
        }))
    }

    /// Tokenizes the input string using the defined mode from TokenizeMode at initialization.
    /// If a mode is provided, that mode will be used temporarily until the end of the function execution.
    /// 
    /// Returns an array of morpheme objects on success, or an error object `{ error: string, details: string }` on failure.
    #[wasm_bindgen]
    pub fn tokenize_raw(&mut self, input: String, mode: Option<TokenizeMode>) -> Result<Vec<TokenMorpheme>, JsValue> {
        self._tokenize(input, mode)
    }
    
    /// Resets the tokenizer, uninitializing it.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.tokenizer = None;
        self.config = None;
        self.morpheme_list = None;
    }

    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        self.tokenizer.is_some()
    }

    /// SplitMode of the tokenizer.
    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> Result<TokenizeMode, JsValue> {
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Tokenizer not initialized. Call initialize() first.".to_string()
            }))?;
        Ok(tokenizer.mode().into())
    }

    #[wasm_bindgen(setter)]
    pub fn set_mode(&mut self, mode: TokenizeMode) -> Result<(), JsValue> {
        let tokenizer = self.tokenizer.as_mut()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Tokenizer not initialized. Call initialize() first.".to_string()
            }))?;
        tokenizer.set_mode(mode.into());
        Ok(())
    }

    /// Debug mode of the tokenizer.
    #[wasm_bindgen(getter)]
    pub fn debug(&self) -> Result<bool, JsValue> {
        let tokenizer = self.tokenizer.as_ref()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Tokenizer not initialized. Call initialize() first.".to_string()
            }))?;
        Ok(tokenizer.debug())
    }

    #[wasm_bindgen(setter)]
    pub fn set_debug(&mut self, debug: bool) -> Result<(), JsValue> {
        let tokenizer = self.tokenizer.as_mut()
            .ok_or_else(|| JsValue::from(WasmError {
                error: "InitializationError".to_string(),
                details: "Tokenizer not initialized. Call initialize() first.".to_string()
            }))?;
        tokenizer.set_debug(debug);
        Ok(())
    }
}
