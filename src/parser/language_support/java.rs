use super::*;
use tree_sitter::{Node, Parser};

pub struct JavaParser {
    parser: Parser,
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_java::language()).unwrap();
        Self { parser }
    }

    #[allow(clippy::only_used_in_recursion)]
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
        // Check if this is a method declaration
        if node.kind() != "method_declaration" && node.kind() != "constructor_declaration" {
            return None;
        }

        // Get method name
        let name = if node.kind() == "method_declaration" {
            node.child_by_field_name("name")?
                .utf8_text(content.as_bytes())
                .ok()?
                .to_string()
        } else {
            // Constructor name comes from the class name
            // We need to find the parent class
            let mut current = node;
            while let Some(parent) = current.parent() {
                if parent.kind() == "class_declaration" {
                    if let Some(class_name) = parent.child_by_field_name("name") {
                        if let Ok(name) = class_name.utf8_text(content.as_bytes()) {
                            return Some(FunctionDefinition {
                                name: name.to_string(),
                                file_path: file_path.to_string(),
                                kind: FunctionKind::Constructor,
                                visibility: self.extract_visibility(node, content),
                                location: self.extract_location(node),
                                containing_type: Some(name.to_string()),
                                parameters: self.extract_parameters(node, content),
                            });
                        }
                    }
                }
                current = parent;
            }
            return None; // Couldn't find parent class
        };

        // Determine containing type
        let mut containing_type = None;
        let mut current = node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "class_declaration" || parent.kind() == "interface_declaration" {
                if let Some(type_name) = parent.child_by_field_name("name") {
                    if let Ok(name) = type_name.utf8_text(content.as_bytes()) {
                        containing_type = Some(name.to_string());
                        break;
                    }
                }
            }
            current = parent;
        }

        Some(FunctionDefinition {
            name,
            file_path: file_path.to_string(),
            kind: if containing_type.is_some() {
                FunctionKind::Method
            } else {
                FunctionKind::Function
            },
            visibility: self.extract_visibility(node, content),
            location: self.extract_location(node),
            containing_type,
            parameters: self.extract_parameters(node, content),
        })
    }

    fn extract_visibility(&self, node: Node, content: &str) -> Visibility {
        // In Java, methods have modifiers like public, private, protected
        if let Some(modifiers) = node.child_by_field_name("modifiers") {
            for i in 0..modifiers.child_count() {
                if let Some(modifier) = modifiers.child(i) {
                    if let Ok(modifier_text) = modifier.utf8_text(content.as_bytes()) {
                        match modifier_text {
                            "private" => return Visibility::Private,
                            "protected" => return Visibility::Protected,
                            "public" => return Visibility::Public,
                            _ => {}
                        }
                    }
                }
            }
        }

        // Default visibility in Java is package-private, map to Protected
        Visibility::Protected
    }

    fn extract_location(&self, node: Node) -> Location {
        Location {
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
        }
    }

    fn extract_parameters(&self, node: Node, content: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            if params_node.kind() == "formal_parameters" {
                for i in 0..params_node.child_count() {
                    if let Some(param_node) = params_node.child(i) {
                        if param_node.kind() == "formal_parameter" {
                            // Get parameter name
                            if let Some(name_node) = param_node.child_by_field_name("name") {
                                if let Ok(param_name) = name_node.utf8_text(content.as_bytes()) {
                                    let mut type_annotation = None;

                                    // Get parameter type
                                    if let Some(type_node) = param_node.child_by_field_name("type")
                                    {
                                        if let Ok(type_text) =
                                            type_node.utf8_text(content.as_bytes())
                                        {
                                            type_annotation = Some(type_text.to_string());
                                        }
                                    }

                                    parameters.push(Parameter {
                                        name: param_name.to_string(),
                                        type_annotation,
                                        default_value: None, // Java doesn't have default parameter values
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        parameters
    }

    fn extract_type_details(
        &self,
        node: Node,
        content: &str,
        file_path: &str,
    ) -> Option<TypeDefinition> {
        if node.kind() != "class_declaration"
            && node.kind() != "interface_declaration"
            && node.kind() != "enum_declaration"
        {
            return None;
        }

        // Get type name
        let name = node
            .child_by_field_name("name")?
            .utf8_text(content.as_bytes())
            .ok()?
            .to_string();

        // Determine type kind
        let kind = match node.kind() {
            "class_declaration" => TypeKind::Class,
            "interface_declaration" => TypeKind::Interface,
            "enum_declaration" => TypeKind::Enum,
            _ => TypeKind::Unknown,
        };

        // Extract super types (extends, implements)
        let mut super_types = Vec::new();

        // Handle extends
        if let Some(extends_node) = node.child_by_field_name("superclass") {
            if let Some(type_node) = extends_node.child(0) {
                if let Ok(type_name) = type_node.utf8_text(content.as_bytes()) {
                    super_types.push(type_name.to_string());
                }
            }
        }

        // Handle implements
        if let Some(implements_node) = node.child_by_field_name("interfaces") {
            for i in 0..implements_node.child_count() {
                if let Some(interface_node) = implements_node.child(i) {
                    if interface_node.kind() == "interface_type_list" {
                        for j in 0..interface_node.child_count() {
                            if let Some(type_node) = interface_node.child(j) {
                                if type_node.kind() == "type_identifier" {
                                    if let Ok(type_name) = type_node.utf8_text(content.as_bytes()) {
                                        super_types.push(type_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Extract visibility
        let visibility = self.extract_type_visibility(node, content);

        // Extract fields
        let fields = self.extract_fields(node, content);

        // Extract method names (for reference)
        let mut methods = Vec::new();
        self.traverse_node(node, &mut |n| {
            if n.kind() == "method_declaration" {
                if let Some(method_name) = n.child_by_field_name("name") {
                    if let Ok(name) = method_name.utf8_text(content.as_bytes()) {
                        methods.push(name.to_string());
                    }
                }
            }
        });

        // Extract documentation comments
        let documentation = self
            .extract_documentation(content, &self.extract_location(node))
            .unwrap_or(None);

        Some(TypeDefinition {
            name,
            file_path: file_path.to_string(),
            kind,
            visibility,
            location: self.extract_location(node),
            super_types,
            fields,
            methods,
            documentation,
        })
    }

    fn extract_type_visibility(&self, node: Node, content: &str) -> Visibility {
        // In Java, classes have modifiers like public, private, protected
        if let Some(modifiers) = node.child_by_field_name("modifiers") {
            for i in 0..modifiers.child_count() {
                if let Some(modifier) = modifiers.child(i) {
                    if let Ok(modifier_text) = modifier.utf8_text(content.as_bytes()) {
                        match modifier_text {
                            "private" => return Visibility::Private,
                            "protected" => return Visibility::Protected,
                            "public" => return Visibility::Public,
                            _ => {}
                        }
                    }
                }
            }
        }

        // Default visibility in Java is package-private, map to Protected
        Visibility::Protected
    }

    fn extract_fields(&self, node: Node, content: &str) -> Vec<FieldDefinition> {
        let mut fields = Vec::new();

        self.traverse_node(node, &mut |n| {
            if n.kind() == "field_declaration" {
                // Java field declaration can contain multiple variables
                if let Some(declarator_list) = n.child_by_field_name("declarator") {
                    // Get the type from the field declaration
                    let mut type_annotation = None;
                    if let Some(type_node) = n.child_by_field_name("type") {
                        if let Ok(type_text) = type_node.utf8_text(content.as_bytes()) {
                            type_annotation = Some(type_text.to_string());
                        }
                    }

                    // Check if field is static
                    let mut is_static = false;
                    if let Some(modifiers) = n.child_by_field_name("modifiers") {
                        for i in 0..modifiers.child_count() {
                            if let Some(modifier) = modifiers.child(i) {
                                if let Ok(modifier_text) = modifier.utf8_text(content.as_bytes()) {
                                    if modifier_text == "static" {
                                        is_static = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // Get visibility
                    let visibility = self.extract_visibility(n, content);

                    // Clone values that will be captured by the closure
                    let vis_clone = visibility.clone();
                    let is_static_clone = is_static;

                    // Process each variable declarator
                    self.traverse_node(declarator_list, &mut |var_node| {
                        if var_node.kind() == "variable_declarator" {
                            if let Some(name_node) = var_node.child_by_field_name("name") {
                                if let Ok(field_name) = name_node.utf8_text(content.as_bytes()) {
                                    // Check for default value
                                    let mut default_value = None;
                                    if let Some(value_node) = var_node.child_by_field_name("value")
                                    {
                                        if let Ok(value_text) =
                                            value_node.utf8_text(content.as_bytes())
                                        {
                                            default_value = Some(value_text.to_string());
                                        }
                                    }

                                    fields.push(FieldDefinition {
                                        name: field_name.to_string(),
                                        type_annotation: type_annotation.clone(),
                                        visibility: vis_clone.clone(),
                                        location: self.extract_location(var_node),
                                        is_static: is_static_clone,
                                        default_value,
                                    });
                                }
                            }
                        }
                    });
                }
            }
        });

        fields
    }
}

impl LanguageParser for JavaParser {
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "java")
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
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Java code"))?;

        let mut functions = Vec::new();
        let root_node = tree.root_node();

        self.traverse_node(root_node, &mut |node| {
            if let Some(func) = self.extract_function_details(node, content, file_path) {
                functions.push(func);
            }
        });

        Ok(functions)
    }

    fn parse_types(&mut self, content: &str, file_path: &str) -> Result<Vec<TypeDefinition>> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Java code"))?;

        let mut types = Vec::new();
        let root_node = tree.root_node();

        self.traverse_node(root_node, &mut |node| {
            if let Some(type_def) = self.extract_type_details(node, content, file_path) {
                types.push(type_def);
            }
        });

        Ok(types)
    }

    fn parse_calls(&mut self, content: &str) -> Result<Vec<CallReference>> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Java code"))?;

        let mut calls = Vec::new();
        let root_node = tree.root_node();

        self.traverse_node(root_node, &mut |node| {
            if node.kind() == "method_invocation" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                        let mut fully_qualified_name = None;

                        // Check if there's an object reference for this method call
                        if let Some(object_node) = node.child_by_field_name("object") {
                            if let Ok(object_name) = object_node.utf8_text(content.as_bytes()) {
                                fully_qualified_name = Some(format!("{}.{}", object_name, name));
                            }
                        }


                        calls.push(CallReference {
                            callee_name: name.to_string(),
                            fully_qualified_name,
                        });
                    }
                }
            }
        });

        Ok(calls)
    }

    fn parse_modules(&mut self, content: &str, file_path: &str) -> Result<ModuleDefinition> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Java code"))?;

        let root_node = tree.root_node();

        // Create a basic module definition
        let mut module_def = ModuleDefinition {
            name: Path::new(file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: file_path.to_string(),
            imports: Vec::new(),
            exports: Vec::new(),
            documentation: None,
        };

        // Extract package name
        for i in 0..root_node.child_count() {
            if let Some(child) = root_node.child(i) {
                if child.kind() == "package_declaration" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(package_name) = name_node.utf8_text(content.as_bytes()) {
                            module_def.name = package_name.to_string();
                        }
                    }
                }
            }
        }

        // Extract imports
        self.traverse_node(root_node, &mut |node| {
            if node.kind() == "import_declaration" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(import_name) = name_node.utf8_text(content.as_bytes()) {
                        let is_relative = import_name.starts_with(".");

                        // Check if it's a wildcard import
                        let mut imported_symbols = Vec::new();
                        if import_name.ends_with(".*") {
                            imported_symbols.push("*".to_string());
                        } else {
                            // Extract the class name from the import
                            if let Some(last_dot) = import_name.rfind('.') {
                                imported_symbols.push(import_name[(last_dot + 1)..].to_string());
                            } else {
                                imported_symbols.push(import_name.to_string());
                            }
                        }

                        module_def.imports.push(ImportDefinition {
                            module_name: import_name.to_string(),
                            imported_symbols,
                            location: self.extract_location(node),
                            is_relative,
                        });
                    }
                }
            }
        });

        // Extract public class/interface names as exports
        self.traverse_node(root_node, &mut |node| {
            if node.kind() == "class_declaration"
                || node.kind() == "interface_declaration"
                || node.kind() == "enum_declaration"
            {
                // Check if the class is public
                let mut is_public = false;
                if let Some(modifiers) = node.child_by_field_name("modifiers") {
                    for i in 0..modifiers.child_count() {
                        if let Some(modifier) = modifiers.child(i) {
                            if let Ok(modifier_text) = modifier.utf8_text(content.as_bytes()) {
                                if modifier_text == "public" {
                                    is_public = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                // If public, add to exports
                if is_public {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                            module_def.exports.push(name.to_string());
                        }
                    }
                }
            }
        });

        Ok(module_def)
    }

    fn extract_documentation(&self, content: &str, location: &Location) -> Result<Option<String>> {
        // In Java, documentation comments start with /** and end with */
        // We need to look for documentation before the entity
        let lines: Vec<&str> = content.lines().collect();

        // The entity starts at this line
        let start_line = location.start.line;

        // Check for out of bounds
        if start_line >= lines.len() || start_line == 0 {
            return Ok(None);
        }

        // Look for documentation comments above the entity
        let mut doc_lines = Vec::new();
        let mut in_doc_comment = false;
        let mut doc_end_line = 0;

        // Safe subset of lines to search
        let search_range = if start_line > 10 { start_line - 10 } else { 0 }..start_line;

        // Iterate through previous lines to find documentation comments
        for i in (search_range).rev() {
            let trimmed = lines[i].trim();

            if trimmed.ends_with("*/") {
                in_doc_comment = true;
                doc_end_line = i;
            } else if trimmed.starts_with("/**") {
                // We found the start of a doc comment block
                if in_doc_comment {
                    // Add this line (with the /** removed) and exit
                    let comment_text = trimmed.strip_prefix("/**").unwrap_or(trimmed).trim();
                    if !comment_text.is_empty() {
                        doc_lines.push(comment_text.to_string());
                    }
                    break;
                }
            } else if in_doc_comment {
                // Inside a doc comment block
                let comment_text = trimmed.strip_prefix("*").unwrap_or(trimmed).trim();
                if !comment_text.is_empty() || !doc_lines.is_empty() {
                    doc_lines.push(comment_text.to_string());
                }
            } else if doc_end_line > 0 && i < doc_end_line - 1
                || (doc_end_line == 0 && !trimmed.is_empty())
            {
                // We've reached non-documentation, non-whitespace content before finding start of comment
                break;
            }
        }

        // Reverse the lines since we collected them in reverse order
        doc_lines.reverse();

        if doc_lines.is_empty() {
            Ok(None)
        } else {
            Ok(Some(doc_lines.join("\n")))
        }
    }

    fn clone_box(&self) -> Box<dyn LanguageParser + Send> {
        Box::new(JavaParser::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_basic_parsing() {
        let java_code = r#"
package com.example;

/**
 * Example class documentation
 */
public class TestClass {
    private String field1;
    
    public TestClass(String initialValue) {
        this.field1 = initialValue;
    }
    
    public void testMethod(String input) {
        System.out.println(input);
    }
}
"#;

        let mut parser = JavaParser::new();

        // Test function parsing
        let functions = parser.parse_functions(java_code, "Test.java").unwrap();
        assert!(functions.len() > 0);

        // Test types parsing
        let types = parser.parse_types(java_code, "Test.java").unwrap();
        assert!(types.len() > 0);

        // Basic module parsing
        let module = parser.parse_modules(java_code, "Test.java").unwrap();
        assert_eq!(module.path, "Test.java");

        // Test that can_handle correctly identifies Java files
        assert!(parser.can_handle(Path::new("test.java")));
        assert!(!parser.can_handle(Path::new("test.py")));
        assert!(!parser.can_handle(Path::new("test.js")));
    }
}
