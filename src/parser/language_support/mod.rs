use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

// Re-export these types for use by language parsers
pub use crate::graph::entity::{Location, Parameter, Position, Visibility};

pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;

// Legacy structures maintained for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub file_path: String,
    pub kind: FunctionKind,
    pub visibility: Visibility,
    pub location: Location,
    pub containing_type: Option<String>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FunctionKind {
    Function,
    Method,
    Constructor,
    Lambda,
    Closure,
}

/// Class/struct definition with fields and methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub file_path: String,
    pub kind: TypeKind,
    pub visibility: Visibility,
    pub location: Location,
    pub super_types: Vec<String>,
    pub fields: Vec<FieldDefinition>,
    pub methods: Vec<String>, // Method names or IDs
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TypeKind {
    Class,
    Struct,
    Interface,
    Trait,
    Enum,
    TypeAlias,
    Union,
    Unknown,
}

/// Field or property definition within a class/struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: Visibility,
    pub location: Location,
    pub is_static: bool,
    pub default_value: Option<String>,
}

/// Module or file representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDefinition {
    pub name: String,
    pub path: String,
    pub imports: Vec<ImportDefinition>,
    pub exports: Vec<String>,
    pub documentation: Option<String>,
}

/// Import statement details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDefinition {
    pub module_name: String,
    pub imported_symbols: Vec<String>,
    pub location: Location,
    pub is_relative: bool,
}

/// Named domain concept extracted from code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConcept {
    pub name: String,
    pub description: Option<String>,
    pub related_entities: Vec<String>,
    pub attributes: Vec<String>,
    pub confidence: f32,
}

static LANGUAGE_PARSERS: Lazy<Mutex<Vec<Box<dyn LanguageParser + Send>>>> = Lazy::new(|| {
    Mutex::new(vec![
        Box::new(rust::RustParser::new()),
        Box::new(python::PythonParser::new()),
        Box::new(javascript::JavaScriptParser::new()),
        Box::new(java::JavaParser::new()),
    ])
});

/// Enhanced language parser trait with support for more code elements
pub trait LanguageParser: Send {
    // Basic file identification
    fn can_handle(&self, file_path: &Path) -> bool;

    // Core parsing methods (original API preserved for compatibility)
    fn parse_functions(
        &mut self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<FunctionDefinition>>;

    fn parse_calls(&mut self, content: &str) -> Result<Vec<CallReference>>;

    // New methods for parsing additional entity types
    fn parse_types(&mut self, _content: &str, _file_path: &str) -> Result<Vec<TypeDefinition>> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }

    fn parse_modules(&mut self, _content: &str, file_path: &str) -> Result<ModuleDefinition> {
        // Default implementation returns basic module info
        let filename = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ModuleDefinition {
            name: filename,
            path: file_path.to_string(),
            imports: Vec::new(),
            exports: Vec::new(),
            documentation: None,
        })
    }

    fn infer_domain_concepts(
        &mut self,
        _content: &str,
        _file_path: &str,
    ) -> Result<Vec<DomainConcept>> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }

    // Extract documentation comments
    fn extract_documentation(
        &self,
        _content: &str,
        _location: &Location,
    ) -> Result<Option<String>> {
        // Default implementation returns None
        Ok(None)
    }

    // Clone method for boxed trait objects
    fn clone_box(&self) -> Box<dyn LanguageParser + Send>;
}

/// Reference to a function/method call site
#[derive(Debug, Clone)]
pub struct CallReference {
    pub callee_name: String,
    pub fully_qualified_name: Option<String>,
}

/// Get the appropriate parser for a given file
pub fn get_parser_for_file(file_path: &Path) -> Option<Box<dyn LanguageParser + Send>> {
    LANGUAGE_PARSERS
        .lock()
        .unwrap()
        .iter()
        .find(|p| p.can_handle(file_path))
        .map(|p| p.clone_box())
}
