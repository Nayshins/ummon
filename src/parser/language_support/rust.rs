use super::{LanguageParser, FunctionDefinition};
use syn;
use anyhow::Result;
use std::path::Path;
use tree_sitter::{Parser, Tree};

pub struct RustParser {
    parser: Parser,
    tree: Option<Tree>,
}

impl RustParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();
        Self { 
            parser,
            tree: None,
        }
    }
}

impl LanguageParser for RustParser {
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    }

    fn parse_functions(&mut self, content: &str) -> Result<Vec<FunctionDefinition>> {
        let tree = self.parser.parse(content, self.tree.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse code"))?;

        let root = tree.root_node();
        let mut out = Vec::new();
        
        fn collect_functions(node: tree_sitter::Node, code: &str, funcs: &mut Vec<FunctionDefinition>) {
            if node.kind() == "function_item" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name_str = &code[name_node.start_byte()..name_node.end_byte()];
                    funcs.push(FunctionDefinition {
                        name: name_str.to_string(),
                        file_path: String::new(), // Will be set by caller
                    });
                }
            }
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    collect_functions(child, code, funcs);
                }
            }
        }

        collect_functions(root, content, &mut out);
        Ok(out)
    }

    fn parse_functions_ast(&self, content: &str) -> Result<Vec<(String, syn::ItemFn)>> {
        // For now we'll return empty vec since we're using tree-sitter instead of syn
        Ok(Vec::new())
    }
}
