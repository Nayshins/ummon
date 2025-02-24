use super::*;
use tree_sitter::{Node, Parser};

pub struct PythonParser {
    parser: Parser,
}

impl PythonParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_python::language()).unwrap();
        Self { parser }
    }

    fn traverse_node<F>(&self, node: Node, f: &mut F)
    where
        F: FnMut(Node),
    {
        f(node);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_node(child, f);
        }
    }

    fn extract_function_details(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
    ) -> Option<FunctionDefinition> {
        if !matches!(node.kind(), "function_definition" | "lambda") {
            return None;
        }

        let name = if node.kind() == "lambda" {
            format!("lambda_{}", node.start_position().row)
        } else {
            node.child_by_field_name("name")?
                .utf8_text(content.as_bytes())
                .ok()?
                .to_string()
        };

        let mut containing_type = None;
        let mut current = node;
        while let Some(p) = current.parent() {
            if p.kind() == "class_definition" {
                if let Some(class_name) = p.child_by_field_name("name") {
                    if let Ok(name) = class_name.utf8_text(content.as_bytes()) {
                        containing_type = Some(name.to_string());
                    }
                }
            }
            current = p;
        }

        Some(FunctionDefinition {
            name: name.clone(),
            file_path: file_path.to_string(),
            kind: if containing_type.is_some() {
                FunctionKind::Method
            } else {
                FunctionKind::Function
            },
            visibility: if name.starts_with("__") {
                Visibility::Private
            } else if name.starts_with("_") {
                Visibility::Protected
            } else {
                Visibility::Public
            },
            location: Location {
                start: Position {
                    line: node.start_position().row,
                    column: node.start_position().column,
                    offset: node.start_byte(),
                },
                end: Position {
                    line: node.end_position().row,
                    column: node.end_position().column,
                    offset: node.end_byte(),
                },
            },
            containing_type,
            parameters: vec![], // TODO: Extract parameters
        })
    }
}

impl LanguageParser for PythonParser {
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "py")
            .unwrap_or(false)
    }

    fn parse_functions(
        &mut self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<FunctionDefinition>> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python code"))?;

        let mut functions = Vec::new();
        let root_node = tree.root_node();

        self.traverse_node(root_node, &mut |node| {
            if let Some(func) = self.extract_function_details(node, content, file_path) {
                functions.push(func);
            }
        });

        Ok(functions)
    }

    fn parse_calls(&mut self, content: &str) -> Result<Vec<CallReference>> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python code"))?;

        let mut calls = Vec::new();
        let root_node = tree.root_node();

        self.traverse_node(root_node, &mut |node| {
            if node.kind() == "call" {
                if let Some(func) = node.child_by_field_name("function") {
                    if let Ok(name) = func.utf8_text(content.as_bytes()) {
                        calls.push(CallReference {
                            caller_location: Location {
                                start: Position {
                                    line: node.start_position().row,
                                    column: node.start_position().column,
                                    offset: node.start_byte(),
                                },
                                end: Position {
                                    line: node.end_position().row,
                                    column: node.end_position().column,
                                    offset: node.end_byte(),
                                },
                            },
                            callee_name: name.to_string(),
                            fully_qualified_name: None,
                            arguments: Vec::new(),
                        });
                    }
                }
            }
        });

        Ok(calls)
    }

    fn clone_box(&self) -> Box<dyn LanguageParser + Send> {
        Box::new(PythonParser::new())
    }
}
