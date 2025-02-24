use super::*;
use anyhow::Result;
use std::path::Path;
use tree_sitter::{Node, Parser};

pub struct JavaScriptParser {
    parser: Parser,
}

impl JavaScriptParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_javascript::language())
            .unwrap();
        Self { parser }
    }

    fn extract_parameters(&self, node: Node, content: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(param_list) = node.child_by_field_name("parameters") {
            let mut cursor = param_list.walk();
            for param_node in param_list.children(&mut cursor) {
                match param_node.kind() {
                    "identifier" => {
                        if let Ok(name) = param_node.utf8_text(content.as_bytes()) {
                            params.push(Parameter {
                                name: name.to_string(),
                                type_annotation: None,
                                default_value: None,
                            });
                        }
                    }
                    "formal_parameter" => {
                        if let Some(param_id) = param_node.child_by_field_name("pattern") {
                            if let Ok(name) = param_id.utf8_text(content.as_bytes()) {
                                let type_annotation = param_node
                                    .child_by_field_name("type")
                                    .and_then(|t| t.utf8_text(content.as_bytes()).ok())
                                    .map(String::from);

                                params.push(Parameter {
                                    name: name.to_string(),
                                    type_annotation,
                                    default_value: None,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        params
    }

    fn extract_function_details(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
    ) -> Option<FunctionDefinition> {
        match node.kind() {
            "function_declaration" | "generator_function_declaration" => {
                let name = node
                    .child_by_field_name("name")?
                    .utf8_text(content.as_bytes())
                    .ok()?
                    .to_string();

                Some(FunctionDefinition {
                    name,
                    file_path: file_path.to_string(),
                    kind: FunctionKind::Function,
                    visibility: Visibility::Default,
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
                    containing_type: None,
                    parameters: self.extract_parameters(node, content),
                })
            }
            "method_definition" => {
                let name = node
                    .child_by_field_name("name")?
                    .utf8_text(content.as_bytes())
                    .ok()?
                    .to_string();

                // Find containing class
                let containing_type = {
                    let mut current = node;
                    let mut result = None;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "class_declaration" {
                            if let Some(class_name) = parent.child_by_field_name("name") {
                                if let Ok(name) = class_name.utf8_text(content.as_bytes()) {
                                    result = Some(name.to_string());
                                }
                            }
                        }
                        current = parent;
                    }
                    result
                };

                let visibility = if name.starts_with('#') {
                    Visibility::Private
                } else {
                    Visibility::Public
                };
                let name_clone = name.clone();

                Some(FunctionDefinition {
                    name,
                    file_path: file_path.to_string(),
                    kind: if name_clone == "constructor" {
                        FunctionKind::Constructor
                    } else {
                        FunctionKind::Method
                    },
                    visibility,
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
                    parameters: self.extract_parameters(node, content),
                })
            }
            "arrow_function" => {
                // For arrow functions, try to find variable assignment name
                let name = {
                    let mut current = node;
                    let mut result = None;
                    while let Some(parent) = current.parent() {
                        if parent.kind() == "variable_declarator" {
                            if let Some(name_node) = parent.child_by_field_name("name") {
                                if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                                    result = Some(format!("arrow_{}", name));
                                }
                            }
                        }
                        current = parent;
                    }
                    result.unwrap_or_else(|| {
                        format!(
                            "arrow_{}_{}",
                            node.start_position().row,
                            node.start_position().column
                        )
                    })
                };

                Some(FunctionDefinition {
                    name,
                    file_path: file_path.to_string(),
                    kind: FunctionKind::Lambda,
                    visibility: Visibility::Default,
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
                    containing_type: None,
                    parameters: self.extract_parameters(node, content),
                })
            }
            _ => None,
        }
    }

    fn extract_call_name(&self, node: Node, content: &str) -> Option<(String, Option<String>)> {
        match node.kind() {
            "identifier" => {
                let name = node.utf8_text(content.as_bytes()).ok()?.to_string();
                Some((name.clone(), Some(name)))
            }
            "member_expression" => {
                let object = node.child_by_field_name("object")?;
                let property = node.child_by_field_name("property")?;

                let obj_text = object.utf8_text(content.as_bytes()).ok()?;
                let prop_text = property.utf8_text(content.as_bytes()).ok()?;

                Some((
                    prop_text.to_string(),
                    Some(format!("{}.{}", obj_text, prop_text)),
                ))
            }
            _ => None,
        }
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
}

impl LanguageParser for JavaScriptParser {
    fn can_handle(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(ext, "js" | "jsx" | "ts" | "tsx")
        } else {
            false
        }
    }

    fn parse_functions(
        &mut self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<FunctionDefinition>> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse JavaScript/TypeScript code"))?;

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
            .ok_or_else(|| anyhow::anyhow!("Failed to parse JavaScript/TypeScript code"))?;

        let mut calls = Vec::new();
        let root_node = tree.root_node();

        self.traverse_node(root_node, &mut |node| {
            if node.kind() == "call_expression" {
                if let Some(function) = node.child_by_field_name("function") {
                    if let Some((name, full_path)) = self.extract_call_name(function, content) {
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
                            callee_name: name,
                            fully_qualified_name: full_path,
                            arguments: Vec::new(),
                        });
                    }
                }
            }
        });

        Ok(calls)
    }

    fn clone_box(&self) -> Box<dyn LanguageParser + Send> {
        Box::new(JavaScriptParser::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function_declaration() -> Result<()> {
        let mut parser = JavaScriptParser::new();
        let content = r#"
            function hello(name) {
                console.log(`Hello, ${name}!`);
            }
        "#;

        let functions = parser.parse_functions(content, "test.js")?;
        assert_eq!(functions.len(), 1);

        let func = &functions[0];
        assert_eq!(func.name, "hello");
        assert_eq!(func.kind, FunctionKind::Function);
        assert_eq!(func.parameters.len(), 1);
        assert_eq!(func.parameters[0].name, "name");

        Ok(())
    }

    #[test]
    fn test_parse_class_method() -> Result<()> {
        let mut parser = JavaScriptParser::new();
        let content = r#"
            class Example {
                constructor(name) {
                    this.name = name;
                }

                greet() {
                    console.log(`Hello, ${this.name}!`);
                }

                #privateMethod() {
                    return 'private';
                }
            }
        "#;

        let functions = parser.parse_functions(content, "test.js")?;
        assert_eq!(functions.len(), 3);

        let constructor = &functions[0];
        assert_eq!(constructor.name, "constructor");
        assert_eq!(constructor.kind, FunctionKind::Constructor);

        let method = &functions[1];
        assert_eq!(method.name, "greet");
        assert_eq!(method.kind, FunctionKind::Method);
        assert_eq!(method.containing_type.as_deref(), Some("Example"));
        assert_eq!(method.visibility, Visibility::Public);

        let private_method = &functions[2];
        assert_eq!(private_method.name, "#privateMethod");
        assert_eq!(private_method.visibility, Visibility::Private);

        Ok(())
    }

    #[test]
    fn test_parse_arrow_function() -> Result<()> {
        let mut parser = JavaScriptParser::new();
        let content = r#"
            const greet = (name) => {
                console.log(`Hello, ${name}!`);
            };
        "#;

        let functions = parser.parse_functions(content, "test.js")?;
        assert_eq!(functions.len(), 1);

        let func = &functions[0];
        assert_eq!(func.kind, FunctionKind::Lambda);
        assert!(func.name.starts_with("arrow_greet"));

        Ok(())
    }

    #[test]
    fn test_parse_calls() -> Result<()> {
        let mut parser = JavaScriptParser::new();
        let content = r#"
            function test() {
                console.log('test');
                someObject.method();
                helper();
            }
        "#;

        let calls = parser.parse_calls(content)?;
        assert_eq!(calls.len(), 3);

        assert_eq!(calls[0].callee_name, "log");
        assert_eq!(
            calls[0].fully_qualified_name.as_deref(),
            Some("console.log")
        );

        assert_eq!(calls[1].callee_name, "method");
        assert_eq!(
            calls[1].fully_qualified_name.as_deref(),
            Some("someObject.method")
        );

        assert_eq!(calls[2].callee_name, "helper");
        assert_eq!(calls[2].fully_qualified_name.as_deref(), Some("helper"));

        Ok(())
    }
}
