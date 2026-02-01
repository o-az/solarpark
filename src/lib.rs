use serde::Serialize;
use solar_ast::Arena;
use solar_interface::{ColorChoice, Session};
use solar_parse::{Lexer, Parser};
use solar_sema::Compiler;
use std::ops::ControlFlow;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[derive(Serialize)]
pub struct ParseResult {
    pub success: bool,
    pub ast: Option<String>,
    pub diagnostics: String,
}

#[derive(Serialize)]
pub struct TokensResult {
    pub success: bool,
    pub tokens: Vec<TokenInfo>,
    pub diagnostics: String,
}

#[derive(Serialize)]
pub struct TokenInfo {
    pub kind: String,
    pub text: String,
    pub span: String,
}

#[derive(Serialize)]
pub struct CompileResult {
    pub success: bool,
    pub contracts: Vec<ContractOutput>,
    pub diagnostics: String,
}

#[derive(Serialize)]
pub struct ContractOutput {
    pub name: String,
    pub abi: serde_json::Value,
    #[serde(rename = "functionHashes")]
    pub function_hashes: std::collections::BTreeMap<String, String>,
    #[serde(rename = "eventHashes")]
    pub event_hashes: std::collections::BTreeMap<String, String>,
    #[serde(rename = "errorHashes")]
    pub error_hashes: std::collections::BTreeMap<String, String>,
    #[serde(rename = "interfaceId")]
    pub interface_id: Option<String>,
}

/// Tokenize Solidity source code and return the token stream.
#[wasm_bindgen]
pub fn tokenize(source: &str) -> String {
    let sess = Session::builder().with_buffer_emitter(ColorChoice::Never).single_threaded().build();

    let result = sess.enter_sequential(|| {
        let file = sess.source_map().new_source_file("input.sol".to_string(), source.to_string());

        let file = match file {
            Ok(f) => f,
            Err(e) => {
                return TokensResult {
                    success: false,
                    tokens: vec![],
                    diagnostics: format!("Failed to create source file: {e}"),
                };
            }
        };

        let lexer = Lexer::from_source_file(&sess, &file);
        let mut tokens = Vec::new();

        for token in lexer {
            let kind = format!("{:?}", token.kind);
            let text = sess.source_map().span_to_snippet(token.span).unwrap_or_default();
            let span = format!("{:?}", token.span);
            tokens.push(TokenInfo { kind, text, span });
        }

        TokensResult { success: true, tokens, diagnostics: String::new() }
    });

    serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

/// Parse Solidity source code and return the AST.
#[wasm_bindgen]
pub fn parse(source: &str) -> String {
    let sess = Session::builder().with_buffer_emitter(ColorChoice::Never).single_threaded().build();

    let result = sess.enter_sequential(|| {
        let arena = Arena::new();
        let file = sess.source_map().new_source_file("input.sol".to_string(), source.to_string());

        let file = match file {
            Ok(f) => f,
            Err(e) => {
                return ParseResult {
                    success: false,
                    ast: None,
                    diagnostics: format!("Failed to create source file: {e}"),
                };
            }
        };

        let lexer = Lexer::from_source_file(&sess, &file);
        let mut parser = Parser::from_lexer(&arena, lexer);

        match parser.parse_file() {
            Ok(ast) => {
                let diagnostics =
                    sess.emitted_diagnostics().map(|d| d.to_string()).unwrap_or_default();
                ParseResult {
                    success: sess.dcx.err_count() == 0,
                    ast: Some(format!("{ast:#?}")),
                    diagnostics,
                }
            }
            Err(e) => {
                e.emit();
                let diagnostics =
                    sess.emitted_diagnostics().map(|d| d.to_string()).unwrap_or_default();
                ParseResult { success: false, ast: None, diagnostics }
            }
        }
    });

    serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

/// Compile Solidity source code and return the ABI for each contract.
#[wasm_bindgen]
pub fn compile(source: &str) -> String {
    let sess = Session::builder().with_buffer_emitter(ColorChoice::Never).single_threaded().build();

    let mut compiler = Compiler::new(sess);

    let result = compiler.enter_sequential_mut(|compiler| {
        // Create source file
        let file = compiler
            .sess()
            .source_map()
            .new_source_file("input.sol".to_string(), source.to_string());

        let file = match file {
            Ok(f) => f,
            Err(e) => {
                return CompileResult {
                    success: false,
                    contracts: vec![],
                    diagnostics: format!("Failed to create source file: {e}"),
                };
            }
        };

        // Parse
        let mut pcx = compiler.parse();
        pcx.add_file(file);
        pcx.parse();

        // Lower to HIR
        let lower_result = compiler.lower_asts();
        if lower_result.is_err() || matches!(lower_result, Ok(ControlFlow::Break(_))) {
            let diagnostics =
                compiler.sess().emitted_diagnostics().map(|d| d.to_string()).unwrap_or_default();
            return CompileResult { success: false, contracts: vec![], diagnostics };
        }

        // Run analysis (type checking)
        if compiler.analysis().is_err() || compiler.sess().dcx.err_count() > 0 {
            let diagnostics =
                compiler.sess().emitted_diagnostics().map(|d| d.to_string()).unwrap_or_default();
            return CompileResult { success: false, contracts: vec![], diagnostics };
        }

        // Extract ABIs, function hashes, event hashes, error hashes, and interface IDs
        let gcx = compiler.gcx();
        let contracts: Vec<ContractOutput> = gcx
            .hir
            .contract_ids()
            .map(|id| {
                let name = gcx.contract_fully_qualified_name(id).to_string();
                let abi = gcx.contract_abi(id);
                let abi_json = serde_json::to_value(&abi).unwrap_or(serde_json::Value::Null);

                // Get function hashes (selectors)
                let mut function_hashes = std::collections::BTreeMap::new();
                for f in gcx.interface_functions(id) {
                    let sig = gcx.item_signature(f.id.into()).to_string();
                    let selector = alloy_primitives::hex::encode(f.selector);
                    function_hashes.insert(sig, selector);
                }

                // Get event hashes (topics)
                let mut event_hashes = std::collections::BTreeMap::new();
                let contract = gcx.hir.contract(id);
                for &item_id in contract.items {
                    if let solar_sema::hir::ItemId::Event(event_id) = item_id {
                        let sig = gcx.item_signature(event_id.into()).to_string();
                        let topic = alloy_primitives::hex::encode(gcx.event_selector(event_id));
                        event_hashes.insert(sig, topic);
                    }
                }

                // Get error hashes (selectors)
                let mut error_hashes = std::collections::BTreeMap::new();
                for &item_id in contract.items {
                    if let solar_sema::hir::ItemId::Error(error_id) = item_id {
                        let sig = gcx.item_signature(error_id.into()).to_string();
                        let selector =
                            alloy_primitives::hex::encode(gcx.function_selector(error_id));
                        error_hashes.insert(sig, selector);
                    }
                }

                // Get interface ID (ERC-165) - only for interfaces
                let interface_id = if contract.kind.is_interface() {
                    Some(alloy_primitives::hex::encode(gcx.interface_id(id)))
                } else {
                    None
                };

                ContractOutput {
                    name,
                    abi: abi_json,
                    function_hashes,
                    event_hashes,
                    error_hashes,
                    interface_id,
                }
            })
            .collect();

        let diagnostics =
            compiler.sess().emitted_diagnostics().map(|d| d.to_string()).unwrap_or_default();

        CompileResult { success: compiler.sess().dcx.err_count() == 0, contracts, diagnostics }
    });

    serde_json::to_string(&result).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

/// Get the Solar version.
#[wasm_bindgen]
pub fn version() -> String {
    solar_config::version::SEMVER_VERSION.to_string()
}
