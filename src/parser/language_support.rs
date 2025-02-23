use anyhow::Result;
use tree_sitter::{Parser, Tree};

#[derive(Debug)]
pub struct FunctionDef {
    pub name: String,
}

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language()).unwrap();
        Self { parser }
    }

    /// Parse code and return top-level fn names
    pub fn parse_functions(&self, code: &str) -> Result<Vec<FunctionDef>> {
        let tree = self
            .parser
            .parse(code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse code"))?;

        let root = tree.root_node();
        let mut out = Vec::new();
        collect_function_items(root, code, &mut out);
        Ok(out)
    }

    /// Return (fn_name, AST node)
    pub fn parse_functions_ast(&self, code: &str) -> Result<Vec<(String, tree_sitter::Node)>> {
        let tree = self
            .parser
            .parse(code, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse code"))?;

        let root = tree.root_node();
        let mut out = Vec::new();
        collect_function_nodes(root, code, &mut out);
        Ok(out)
    }
}

fn collect_function_items(node: tree_sitter::Node, code: &str, funcs: &mut Vec<FunctionDef>) {
    if node.kind() == "function_item" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name_str = &code[name_node.start_byte()..name_node.end_byte()];
            funcs.push(FunctionDef {
                name: name_str.to_string(),
            });
        }
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_function_items(child, code, funcs);
        }
    }
}

fn collect_function_nodes(
    node: tree_sitter::Node,
    code: &str,
    funcs: &mut Vec<(String, tree_sitter::Node)>,
) {
    if node.kind() == "function_item" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name_str = &code[name_node.start_byte()..name_node.end_byte()];
            funcs.push((name_str.to_string(), node));
        }
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_function_nodes(child, code, funcs);
        }
    }
}
