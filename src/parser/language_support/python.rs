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

        // Extract parameters
        let mut parameters = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            // Process function parameters - looking at the structure we saw in the debug output
            // Each parameter is separated by commas and may have different types (identifier, default_parameter, etc.)
            if params_node.kind() == "parameters" {
                // Regular function parameters
                for i in 0..params_node.child_count() {
                    if let Some(child) = params_node.child(i) {
                        match child.kind() {
                            // Regular parameter without type or default
                            "identifier" => {
                                if let Ok(param_name) = child.utf8_text(content.as_bytes()) {
                                    parameters.push(Parameter {
                                        name: param_name.to_string(),
                                        type_annotation: None,
                                        default_value: None,
                                    });
                                }
                            }
                            // Parameter with default value (b=10)
                            "default_parameter" => {
                                if let Some(id_node) = child.child(0) {
                                    if let Ok(param_name) = id_node.utf8_text(content.as_bytes()) {
                                        let mut default_value = None;

                                        // Get the default value (after the = sign)
                                        if let Some(value_node) = child.child(2) {
                                            if let Ok(value_text) =
                                                value_node.utf8_text(content.as_bytes())
                                            {
                                                default_value = Some(value_text.to_string());
                                            }
                                        }

                                        parameters.push(Parameter {
                                            name: param_name.to_string(),
                                            type_annotation: None,
                                            default_value,
                                        });
                                    }
                                }
                            }
                            // Parameter with type annotation (param2: int)
                            "typed_parameter" => {
                                if let Some(id_node) = child.child(0) {
                                    if let Ok(param_name) = id_node.utf8_text(content.as_bytes()) {
                                        let mut type_annotation = None;

                                        // Get the type annotation (after the : symbol)
                                        if let Some(type_node) = child.child(2) {
                                            if let Ok(type_text) =
                                                type_node.utf8_text(content.as_bytes())
                                            {
                                                type_annotation = Some(type_text.to_string());
                                            }
                                        }

                                        parameters.push(Parameter {
                                            name: param_name.to_string(),
                                            type_annotation,
                                            default_value: None,
                                        });
                                    }
                                }
                            }
                            // Parameter with both type and default (c: str = "default")
                            "typed_default_parameter" => {
                                if let Some(id_node) = child.child(0) {
                                    if let Ok(param_name) = id_node.utf8_text(content.as_bytes()) {
                                        let mut type_annotation = None;
                                        let mut default_value = None;

                                        // Get the type annotation
                                        if let Some(type_node) = child.child(2) {
                                            if let Ok(type_text) =
                                                type_node.utf8_text(content.as_bytes())
                                            {
                                                type_annotation = Some(type_text.to_string());
                                            }
                                        }

                                        // Get the default value
                                        if let Some(value_node) = child.child(4) {
                                            if let Ok(value_text) =
                                                value_node.utf8_text(content.as_bytes())
                                            {
                                                default_value = Some(value_text.to_string());
                                            }
                                        }

                                        parameters.push(Parameter {
                                            name: param_name.to_string(),
                                            type_annotation,
                                            default_value,
                                        });
                                    }
                                }
                            }
                            // Handle *args parameters
                            "list_splat_pattern" => {
                                // Get the name after the '*'
                                if let Some(id_node) = child.child(1) {
                                    if let Ok(param_name) = id_node.utf8_text(content.as_bytes()) {
                                        parameters.push(Parameter {
                                            name: format!("*{}", param_name),
                                            type_annotation: None,
                                            default_value: None,
                                        });
                                    }
                                }
                            }
                            // Handle **kwargs parameters
                            "dictionary_splat_pattern" => {
                                // Get the name after the '**'
                                if let Some(id_node) = child.child(1) {
                                    if let Ok(param_name) = id_node.utf8_text(content.as_bytes()) {
                                        parameters.push(Parameter {
                                            name: format!("**{}", param_name),
                                            type_annotation: None,
                                            default_value: None,
                                        });
                                    }
                                }
                            }
                            _ => {} // Ignore commas, parentheses, etc.
                        }
                    }
                }
            } else if params_node.kind() == "lambda_parameters" {
                // Lambda function parameters
                for i in 0..params_node.child_count() {
                    if let Some(param_node) = params_node.child(i) {
                        if param_node.kind() == "identifier" {
                            if let Ok(param_name) = param_node.utf8_text(content.as_bytes()) {
                                parameters.push(Parameter {
                                    name: param_name.to_string(),
                                    type_annotation: None,
                                    default_value: None,
                                });
                            }
                        }
                    }
                }
            }
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
            parameters,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_function_parameter_extraction() {
        let python_code = r#"
def simple_function(a, b=10, c: str = "default"):
    return a + b

class TestClass:
    def method_with_self(self, param1, param2: int):
        return param1 + param2
        
    @classmethod
    def class_method(cls, data):
        return data
        
def function_with_args(a, *args, **kwargs):
    return a

lambda_func = lambda x, y: x + y
"#;

        let mut parser = PythonParser::new();
        let results = parser.parse_functions(python_code, "test.py").unwrap();

        // We expect specific functions with specific names
        assert!(results.iter().any(|f| f.name == "simple_function"));
        assert!(results.iter().any(|f| f.name == "method_with_self"));
        assert!(results.iter().any(|f| f.name == "class_method"));
        assert!(results.iter().any(|f| f.name == "function_with_args"));

        // Find each function
        let simple_func = results
            .iter()
            .find(|f| f.name == "simple_function")
            .unwrap();
        let method = results
            .iter()
            .find(|f| f.name == "method_with_self")
            .unwrap();
        let class_method = results.iter().find(|f| f.name == "class_method").unwrap();
        let args_func = results
            .iter()
            .find(|f| f.name == "function_with_args")
            .unwrap();

        // Test simple function parameters
        assert_eq!(simple_func.name, "simple_function");
        assert_eq!(simple_func.parameters.len(), 3);
        assert_eq!(simple_func.parameters[0].name, "a");
        assert_eq!(simple_func.parameters[0].default_value, None);
        assert_eq!(simple_func.parameters[1].name, "b");
        assert_eq!(
            simple_func.parameters[1].default_value,
            Some("10".to_string())
        );
        assert_eq!(simple_func.parameters[2].name, "c");
        assert_eq!(
            simple_func.parameters[2].type_annotation,
            Some("str".to_string())
        );
        assert_eq!(
            simple_func.parameters[2].default_value,
            Some("\"default\"".to_string())
        );

        // Test class method parameters
        assert_eq!(method.name, "method_with_self");
        assert_eq!(method.containing_type, Some("TestClass".to_string()));
        assert_eq!(method.parameters.len(), 3);
        assert_eq!(method.parameters[0].name, "self");
        assert_eq!(method.parameters[1].name, "param1");
        assert_eq!(method.parameters[2].name, "param2");
        assert_eq!(
            method.parameters[2].type_annotation,
            Some("int".to_string())
        );

        // Test classmethod parameters
        assert_eq!(class_method.name, "class_method");
        assert_eq!(class_method.parameters.len(), 2);
        assert_eq!(class_method.parameters[0].name, "cls");

        // Test *args and **kwargs
        assert_eq!(args_func.name, "function_with_args");
        let param_names: Vec<&str> = args_func
            .parameters
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(param_names.contains(&"a"));
        assert!(param_names.contains(&"*args"));
        assert!(param_names.contains(&"**kwargs"));

        // Test lambda parameters
        let lambda = results
            .iter()
            .find(|f| f.name.starts_with("lambda_"))
            .unwrap();
        assert_eq!(lambda.parameters.len(), 2);
        assert_eq!(lambda.parameters[0].name, "x");
        assert_eq!(lambda.parameters[1].name, "y");
    }
}
