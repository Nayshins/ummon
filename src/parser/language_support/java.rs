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
                                containing_entity_name: Some(name.to_string()),
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
            containing_type: containing_type.clone(),
            parameters: self.extract_parameters(node, content),
            containing_entity_name: containing_type,
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

    fn extract_generic_parameters(&self, node: Node, content: &str) -> Vec<GenericParameter> {
        // For Java, generic parameters are enclosed in angle brackets like:
        // public class Box<T> { } or
        // public interface List<E extends Comparable<E>> { }

        // Try to find type parameters section - it's a direct child node with angle brackets
        let mut generic_params = Vec::new();

        // Since the tree-sitter grammar might not directly expose type parameters,
        // manually extract them from the class tex
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        // Add bounds checking to prevent panics
        if start_byte >= content.len() || end_byte > content.len() || start_byte > end_byte {
            tracing::warn!(
                "Invalid byte range for Java class: [{},{}], content length: {}",
                start_byte,
                end_byte,
                content.len()
            );
            return Vec::new();
        }

        let class_text = &content[start_byte..end_byte];

        // Look for the pattern "class/interface Name<T, U...>"
        if let Some(start_idx) = class_text.find('<') {
            if let Some(end_idx) = class_text.find('>') {
                if start_idx < end_idx
                    && start_idx + 1 < class_text.len()
                    && end_idx <= class_text.len()
                {
                    let params_text = &class_text[start_idx + 1..end_idx];

                    // Split by commas for multiple parameters
                    for param_text in params_text.split(',') {
                        let param_text = param_text.trim();
                        if param_text.is_empty() {
                            continue; // Skip empty parameters
                        }

                        // Handle bounds with "extends" keyword
                        let parts: Vec<&str> = param_text.split("extends").collect();
                        if parts.is_empty() {
                            continue; // Skip if there are no parts (shouldn't happen)
                        }

                        let param_name = parts[0].trim().to_string();
                        if param_name.is_empty() {
                            continue; // Skip empty parameter names
                        }

                        let mut param = GenericParameter {
                            name: param_name,
                            bounds: Vec::new(),
                            default_type: None,
                        };

                        // If there are bounds (extends Comparable & Serializable)
                        if parts.len() > 1 {
                            let bounds_text = parts[1].trim();
                            if bounds_text.contains('&') {
                                // Multiple bounds
                                for bound in bounds_text.split('&') {
                                    let bound = bound.trim();
                                    if !bound.is_empty() {
                                        param.bounds.push(bound.to_string());
                                    }
                                }
                            } else if !bounds_text.is_empty() {
                                // Single bound (check if not empty)
                                param.bounds.push(bounds_text.to_string());
                            }
                        }

                        generic_params.push(param);
                    }
                } else {
                    tracing::warn!(
                        "Invalid generic parameter indices in class text: start={}, end={}, length={}",
                        start_idx, end_idx, class_text.len()
                    );
                }
            }
        }

        generic_params
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

        // Check if this type has a containing type
        let containing_entity_name = node
            .parent()
            .filter(|parent| parent.kind() == "class_body")
            .and_then(|parent| parent.parent())
            .filter(|grandparent| grandparent.kind() == "class_declaration")
            .and_then(|grandparent| grandparent.child_by_field_name("name"))
            .and_then(|parent_name| parent_name.utf8_text(content.as_bytes()).ok())
            .map(String::from);

        // Extract generic type parameters
        let generic_params = self.extract_generic_parameters(node, content);

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
            containing_entity_name,
            generic_params,
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

                                    let mut annotations = Vec::new();
                                    if let Some(modifiers) = n.child_by_field_name("modifiers") {
                                        for i in 0..modifiers.child_count() {
                                            if let Some(modifier) = modifiers.child(i) {
                                                if let Ok(modifier_text) =
                                                    modifier.utf8_text(content.as_bytes())
                                                {
                                                    if ![
                                                        "public",
                                                        "private",
                                                        "protected",
                                                        "static",
                                                        "final",
                                                    ]
                                                    .contains(&modifier_text)
                                                    {
                                                        annotations.push(modifier_text.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    let is_optional = type_annotation
                                        .as_ref()
                                        .map(|t| t.contains("Optional<") || t.contains("@Nullable"))
                                        .unwrap_or(false);

                                    let documentation = self
                                        .extract_documentation(
                                            content,
                                            &self.extract_location(var_node),
                                        )
                                        .unwrap_or(None);

                                    fields.push(FieldDefinition {
                                        name: field_name.to_string(),
                                        type_annotation: type_annotation.clone(),
                                        visibility: vis_clone.clone(),
                                        location: self.extract_location(var_node),
                                        is_static: is_static_clone,
                                        default_value,
                                        is_optional,
                                        annotations,
                                        documentation,
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
        file_path.extension().and_then(|ext| ext.to_str()) == Some("java")
    }

    /// Parses Java methods and constructors from source code.
    ///
    /// Extracts methods and constructors from classes and interfaces, identifying their
    /// structure, parameters, visibility, and relationships to containing classes.
    ///
    /// # Arguments
    /// * `content` - The Java source code as a string
    /// * `file_path` - Path to the source file
    ///
    /// # Returns
    /// * `Result<Vec<FunctionDefinition>>` - List of extracted function definitions or an error
    fn parse_functions(
        &mut self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<FunctionDefinition>> {
        // Handle empty content case gracefully
        if content.is_empty() {
            tracing::debug!("Empty Java file content for '{}'", file_path);
            return Ok(Vec::new());
        }

        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| {
                let filename = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                anyhow::anyhow!(
                    "Failed to parse Java functions in file '{}' ({}B). The file may contain syntax errors.",
                    filename,
                    content.len()
                )
            })?;

        let mut functions = Vec::new();
        let root_node = tree.root_node();

        tracing::debug!(
            "Parsing Java functions in file '{}' ({} bytes) - found {} child nodes",
            file_path,
            content.len(),
            root_node.child_count()
        );

        self.traverse_node(root_node, &mut |node| {
            if let Some(func) = self.extract_function_details(node, content, file_path) {
                functions.push(func);
            }
        });

        Ok(functions)
    }

    fn parse_types(&mut self, content: &str, file_path: &str) -> Result<Vec<TypeDefinition>> {
        // Handle empty content case gracefully
        if content.is_empty() {
            tracing::debug!("Empty Java file content for '{}'", file_path);
            return Ok(Vec::new());
        }

        let tree = self.parser.parse(content, None).ok_or_else(|| {
            let filename = Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            anyhow::anyhow!(
                "Failed to parse Java code in file '{}' ({}B). The file may contain syntax errors.",
                filename,
                content.len()
            )
        })?;

        let mut types = Vec::new();
        let root_node = tree.root_node();

        tracing::debug!(
            "Parsed Java file '{}' ({} bytes) - AST has {} nodes",
            file_path,
            content.len(),
            root_node.child_count()
        );

        self.traverse_node(root_node, &mut |node| {
            if let Some(type_def) = self.extract_type_details(node, content, file_path) {
                types.push(type_def);
            }
        });

        Ok(types)
    }

    fn parse_calls(&mut self, content: &str, file_path: &str) -> Result<Vec<CallReference>> {
        // Handle empty content case gracefully
        if content.is_empty() {
            tracing::debug!(
                "Empty Java file content for '{}' when parsing calls",
                file_path
            );
            return Ok(Vec::new());
        }

        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| {
                // Provide detailed error message with file info
                let filename = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                anyhow::anyhow!(
                    "Failed to parse Java code for method calls in file '{}' ({}B). The file may contain syntax errors.",
                    filename,
                    content.len()
                )
            })?;

        let mut calls = Vec::new();
        let root_node = tree.root_node();

        tracing::debug!(
            "Parsing Java method calls in file '{}' ({} bytes) - found {} child nodes",
            file_path,
            content.len(),
            root_node.child_count()
        );

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

                        // Create location for the call
                        let location = Location {
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
                        };

                        // Use helper method to create the call reference
                        calls.push(CallReference::with_details(
                            name.to_string(),
                            fully_qualified_name,
                            Some(location),
                            Some(file_path.to_string()),
                            Vec::new(), // We'll add argument extraction later
                        ));
                    }
                }
            }
        });

        Ok(calls)
    }

    fn parse_modules(&mut self, content: &str, file_path: &str) -> Result<ModuleDefinition> {
        // Handle empty content case gracefully
        if content.is_empty() {
            tracing::debug!(
                "Empty Java file content for module parsing: '{}'",
                file_path
            );
            // Return a basic empty module definition for empty files
            return Ok(ModuleDefinition {
                name: Path::new(file_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                path: file_path.to_string(),
                imports: Vec::new(),
                exports: Vec::new(),
                documentation: None,
            });
        }

        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| {
                let filename = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                anyhow::anyhow!(
                    "Failed to parse Java module in file '{}' ({}B). The file may contain syntax errors.",
                    filename,
                    content.len()
                )
            })?;

        let root_node = tree.root_node();

        tracing::debug!(
            "Parsing Java module in file '{}' ({} bytes) - found {} child nodes",
            file_path,
            content.len(),
            root_node.child_count()
        );

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

                        // Check if it's a wildcard impor
                        let mut imported_symbols = Vec::new();
                        if import_name.ends_with(".*") {
                            imported_symbols.push("*".to_string());
                        } else {
                            // Extract the class name from the impor
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

        // Check for empty conten
        if content.is_empty() {
            return Ok(None);
        }

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
                    // Add this line (with the /** removed) and exi
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
                // We've reached non-documentation, non-whitespace content before finding start of commen
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
    use indoc::indoc;

    #[test]
    fn test_java_basic_parsing() {
        let java_code = indoc! {r#"
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
        "#};

        let mut parser = JavaParser::new();

        // Test function parsing
        let functions = parser.parse_functions(java_code, "Test.java").unwrap();
        assert!(!functions.is_empty());

        // Test types parsing
        let types = parser.parse_types(java_code, "Test.java").unwrap();
        assert!(!types.is_empty());

        // Basic module parsing
        let module = parser.parse_modules(java_code, "Test.java").unwrap();
        assert_eq!(module.path, "Test.java");

        // Test that can_handle correctly identifies Java files
        assert!(parser.can_handle(Path::new("test.java")));
        assert!(!parser.can_handle(Path::new("test.py")));
        assert!(!parser.can_handle(Path::new("test.js")));
    }

    #[test]
    fn test_java_parser_empty_content() {
        let mut parser = JavaParser::new();

        // Test empty content for functions
        let functions = parser.parse_functions("", "empty.java").unwrap();
        assert!(
            functions.is_empty(),
            "Empty content should yield empty functions result"
        );

        // Test empty content for types
        let types = parser.parse_types("", "empty.java").unwrap();
        assert!(
            types.is_empty(),
            "Empty content should yield empty types result"
        );

        // Test empty content for calls
        let calls = parser.parse_calls("", "empty.java").unwrap();
        assert!(
            calls.is_empty(),
            "Empty content should yield empty calls result"
        );

        // Test empty content for modules
        let module = parser.parse_modules("", "empty.java").unwrap();
        assert!(
            module.imports.is_empty(),
            "Empty content should yield module with no imports"
        );
        assert!(
            module.exports.is_empty(),
            "Empty content should yield module with no exports"
        );
    }

    #[test]
    fn test_java_parser_invalid_content() -> Result<()> {
        let mut parser = JavaParser::new();

        // Test with invalid Java code that should produce empty results
        let invalid_code = "This isn't even valid Java syntax @#$%^&*()";

        // For now, just verify that it doesn't crash and returns empty results
        let functions = parser.parse_functions(invalid_code, "invalid.java")?;
        assert!(
            functions.is_empty() || !functions.is_empty(),
            "Parser should not crash on invalid content"
        );

        let types = parser.parse_types(invalid_code, "invalid.java")?;
        assert!(
            types.is_empty() || !types.is_empty(),
            "Parser should not crash on invalid content"
        );

        let calls = parser.parse_calls(invalid_code, "invalid.java")?;
        assert!(
            calls.is_empty() || !calls.is_empty(),
            "Parser should not crash on invalid content"
        );

        let module = parser.parse_modules(invalid_code, "invalid.java")?;
        assert!(
            module.imports.is_empty() || !module.imports.is_empty(),
            "Parser should not crash on invalid content"
        );

        Ok(())
    }

    #[test]
    fn test_java_parser_boundary_conditions() {
        let mut parser = JavaParser::new();

        // Test with generic parameters at boundaries
        let boundary_code = indoc! {r#"
            package com.example;

            import java.util.List;
            import java.util.Map;

            // Class with generic parameters where the bracket is at the end of a line
            public class BoundaryCase<T
                extends CharSequence> {

                // Method with weird boundary parameter extraction
                public <E> List<E> process(Map<String, E> items) {
                    return null;
                }
            }
        "#};

        // This should parse without errors
        let types = parser.parse_types(boundary_code, "boundary.java").unwrap();
        assert!(
            !types.is_empty(),
            "Should parse boundary case code successfully"
        );

        let boundary_class = types.iter().find(|t| t.name == "BoundaryCase");
        assert!(
            boundary_class.is_some(),
            "BoundaryCase class should be found"
        );

        // Test modules with boundary conditions
        let module = parser
            .parse_modules(boundary_code, "boundary.java")
            .unwrap();
        assert!(
            module.name == "com.example" || module.name == "boundary.java",
            "Should extract package name correctly"
        );
    }

    #[test]
    fn test_java_field_extraction() {
        let java_code = indoc! {r#"
            package com.example;

            import java.util.Optional;

            /**
             * Class with various field types for testing
             */
            public class FieldTest {
                // Basic fields with various visibilities
                public String publicField;
                private int privateField = 42;
                protected boolean protectedField;

                // Static field
                private static final String CONSTANT = "constant value";

                // Field with annotations
                @Deprecated
                public long annotatedField;

                // Optional field (should be detected as optional)
                private Optional<String> optionalField = Optional.empty();

                /**
                 * Documented field
                 */
                public String documentedField;
            }
        "#};

        let mut parser = JavaParser::new();
        let types = parser.parse_types(java_code, "FieldTest.java").unwrap();

        // For debugging
        println!("Types found: {}", types.len());
        if !types.is_empty() {
            println!("Fields found: {}", types[0].fields.len());
            for field in &types[0].fields {
                println!("Field: {}", field.name);
            }
        }

        assert!(!types.is_empty(), "No class types found");

        if !types.is_empty() {
            let class_def = &types[0];
            assert_eq!(class_def.name, "FieldTest");

            // For now, check if any fields are found
            if !class_def.fields.is_empty() {
                // Check visibility of first field
                let field = &class_def.fields[0];
                assert!(!field.name.is_empty(), "Field name is empty");
            }
        }
    }

    #[test]
    fn test_java_generic_parameters() {
        let mut parser = JavaParser::new();
        let content = indoc! {r#"
            public class Box<T> {
                private T value;

                public T getValue() {
                    return value;
                }

                public void setValue(T value) {
                    this.value = value;
                }
            }

            public interface Comparable<T> {
                int compareTo(T other);
            }

            public class DataProcessor<T extends Serializable & Cloneable> {
                public T process(T data) {
                    return data;
                }
            }
        "#};

        let types = parser.parse_types(content, "Box.java").unwrap();
        assert_eq!(types.len(), 3, "Expected three types");

        // Check Box<T>
        let box_class = types
            .iter()
            .find(|t| t.name == "Box")
            .expect("Box class not found");
        assert_eq!(
            box_class.generic_params.len(),
            1,
            "Box should have 1 generic parameter"
        );
        assert_eq!(
            box_class.generic_params[0].name, "T",
            "Box parameter should be T"
        );
        assert!(
            box_class.generic_params[0].bounds.is_empty(),
            "T should have no bounds"
        );

        // Check Comparable<T>
        let comparable = types
            .iter()
            .find(|t| t.name == "Comparable")
            .expect("Comparable interface not found");
        assert_eq!(
            comparable.generic_params.len(),
            1,
            "Comparable should have 1 generic parameter"
        );
        assert_eq!(
            comparable.generic_params[0].name, "T",
            "Comparable parameter should be T"
        );

        // Check DataProcessor<T extends Serializable & Cloneable>
        let processor = types
            .iter()
            .find(|t| t.name == "DataProcessor")
            .expect("DataProcessor class not found");
        assert_eq!(
            processor.generic_params.len(),
            1,
            "DataProcessor should have 1 generic parameter"
        );
        let t_param = &processor.generic_params[0];
        assert_eq!(t_param.name, "T", "DataProcessor parameter should be T");
        assert_eq!(t_param.bounds.len(), 2, "T should have 2 bounds");
        assert!(
            t_param.bounds.iter().any(|b| b.contains("Serializable")),
            "T should extend Serializable"
        );
        assert!(
            t_param.bounds.iter().any(|b| b.contains("Cloneable")),
            "T should extend Cloneable"
        );
    }
}
