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
    pub containing_type: Option<String>, // Kept for backward compatibility
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub containing_entity_name: Option<String>, // Name of the parent entity (could be a type, module, or function)
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
    #[serde(default)]
    pub containing_entity_name: Option<String>, // Name of the parent entity (if nested)
    #[serde(default)]
    pub generic_params: Vec<GenericParameter>, // Generic type parameters
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

/// Field or property definition within a class/struc
/// Includes metadata like optionality, annotations, and documentation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldDefinition {
    pub name: String,
    pub type_annotation: Option<String>,
    pub visibility: Visibility,
    pub location: Location,
    pub is_static: bool,
    pub default_value: Option<String>,
    #[serde(default)]
    pub is_optional: bool,
    #[serde(default)]
    pub annotations: Vec<String>,
    #[serde(default)]
    pub documentation: Option<String>,
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

/// Generic type parameter definition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenericParameter {
    pub name: String,
    pub bounds: Vec<String>, // Trait or interface bounds (e.g., T: Display + Debug)
    pub default_type: Option<String>, // Default type if specified (e.g., T = String)
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

/// Standardized language parser trait with consistent interface and error handling
pub trait LanguageParser: Send {
    // ---- FILE IDENTIFICATION ----

    /// Determines if this parser can handle a given file based on extension and conten
    ///
    /// # Arguments
    /// * `file_path` - Path to the file to check
    ///
    /// # Returns
    /// * `bool` - True if this parser can handle the file, false otherwise
    fn can_handle(&self, file_path: &Path) -> bool;

    // ---- CORE PARSING METHODS ----

    /// Parses functions and methods from source code
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `file_path` - Path to the source file (for error reporting and context)
    ///
    /// # Returns
    /// * `Result<Vec<FunctionDefinition>>` - List of extracted function definitions or an error
    ///
    /// # Implementation Requirements
    /// * Must handle empty content by returning an empty vector
    /// * Must provide meaningful error messages with file contex
    /// * Should validate string bounds before operations
    /// * Should use tracing for logging parsing statistics
    fn parse_functions(
        &mut self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<FunctionDefinition>>;

    /// Parses function and method calls from source code
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `file_path` - Path to the source file (for error reporting and context)
    ///
    /// # Returns
    /// * `Result<Vec<CallReference>>` - List of extracted function call references or an error
    ///
    /// # Implementation Requirements
    /// * Must handle empty content by returning an empty vector
    /// * Must provide meaningful error messages with file contex
    /// * Should validate string bounds before operations
    /// * Should use tracing for logging parsing statistics
    fn parse_calls(&mut self, content: &str, file_path: &str) -> Result<Vec<CallReference>>;

    // ---- EXTENDED PARSING METHODS ----

    /// Parses types (classes, structs, interfaces, etc.) from source code
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `file_path` - Path to the source file (for error reporting and context)
    ///
    /// # Returns
    /// * `Result<Vec<TypeDefinition>>` - List of extracted type definitions or an error
    ///
    /// # Implementation Requirements
    /// * Must handle empty content by returning an empty vector
    /// * Must provide meaningful error messages with file contex
    /// * Should validate string bounds before operations
    /// * Should use tracing for logging parsing statistics
    fn parse_types(&mut self, content: &str, file_path: &str) -> Result<Vec<TypeDefinition>> {
        // Default implementation returns empty lis
        if content.is_empty() {
            tracing::debug!("Empty file content for '{}'", file_path);
            return Ok(Vec::new());
        }

        tracing::debug!("Type parsing not implemented for file '{}'", file_path);
        Ok(Vec::new())
    }

    /// Parses module information from source code
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `file_path` - Path to the source file (for error reporting and context)
    ///
    /// # Returns
    /// * `Result<ModuleDefinition>` - Module definition or an error
    ///
    /// # Implementation Requirements
    /// * Must handle empty content appropriately
    /// * Must provide meaningful error messages with file contex
    /// * Should validate string bounds before operations
    /// * Should use tracing for logging parsing statistics
    fn parse_modules(&mut self, content: &str, file_path: &str) -> Result<ModuleDefinition> {
        // Default implementation returns basic module info
        if content.is_empty() {
            tracing::debug!("Empty file content for '{}'", file_path);
        }

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

    /// Infers domain concepts from code
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `file_path` - Path to the source file (for error reporting and context)
    ///
    /// # Returns
    /// * `Result<Vec<DomainConcept>>` - List of inferred domain concepts or an error
    ///
    /// # Implementation Requirements
    /// * Must handle empty content by returning an empty vector
    /// * Should validate string bounds before operations
    fn infer_domain_concepts(
        &mut self,
        content: &str,
        file_path: &str,
    ) -> Result<Vec<DomainConcept>> {
        // Default implementation returns empty lis
        if content.is_empty() {
            tracing::debug!("Empty file content for '{}'", file_path);
        }
        Ok(Vec::new())
    }

    // ---- UTILITY METHODS ----

    /// Extracts documentation comments for a code elemen
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `location` - Source location of the code elemen
    ///
    /// # Returns
    /// * `Result<Option<String>>` - Extracted documentation or None if not found
    ///
    /// # Implementation Requirements
    /// * Must handle empty content by returning None
    /// * Should validate string bounds before operations
    fn extract_documentation(&self, content: &str, _location: &Location) -> Result<Option<String>> {
        // Default implementation returns None
        if content.is_empty() {
            tracing::debug!("Empty content provided to extract_documentation");
        }
        Ok(None)
    }

    /// Safely extracts text from source code with bounds checking
    ///
    /// # Arguments
    /// * `content` - The source code as a string
    /// * `start` - Starting byte offse
    /// * `end` - Ending byte offse
    ///
    /// # Returns
    /// * `Option<String>` - Extracted text or None if out of bounds
    fn safe_extract_text(&self, content: &str, start: usize, end: usize) -> Option<String> {
        if start < content.len() && end <= content.len() && start < end {
            Some(content[start..end].to_string())
        } else {
            tracing::warn!(
                "Invalid byte range: {}..{} (content length: {})",
                start,
                end,
                content.len()
            );
            None
        }
    }

    // Clone method for boxed trait objects
    fn clone_box(&self) -> Box<dyn LanguageParser + Send>;
}

/// Reference to a function/method call site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallReference {
    /// Name of the called function or method
    pub callee_name: String,

    /// Fully qualified name if available (e.g., module.submodule.function)
    pub fully_qualified_name: Option<String>,

    /// Source location of the call site
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// File path where the call was found
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    /// Arguments passed to the function (if available)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<String>,
}

impl CallReference {
    /// Creates a new CallReference with all fields
    pub fn with_details(
        callee_name: String,
        fully_qualified_name: Option<String>,
        location: Option<Location>,
        file_path: Option<String>,
        arguments: Vec<String>,
    ) -> Self {
        Self {
            callee_name,
            fully_qualified_name,
            location,
            file_path,
            arguments,
        }
    }
}

/// Get the appropriate parser for a given file
/// Gets a parser that can handle the given file type
///
/// # Arguments
/// * `file_path` - Path to the file that needs parsing
///
/// # Returns
/// * `Result<Option<Box<dyn LanguageParser + Send>>>` - A parser if one is found for the file extension
pub fn get_parser_for_file(file_path: &Path) -> Result<Option<Box<dyn LanguageParser + Send>>> {
    // Get file extension if possible for error reporting
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    // Safely lock the mutex
    LANGUAGE_PARSERS
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to access language parsers: {}", e))?
        .iter()
        .find(|p| p.can_handle(file_path))
        .map(|p| p.clone_box())
        .map_or_else(
            || {
                // Log that no parser was found for this file type
                tracing::debug!("No parser found for file type: {}", extension);
                Ok(None)
            },
            |parser| Ok(Some(parser)),
        )
}
