use anyhow::Result;
use std::path::Path;

pub mod rust;
pub mod python;

pub trait LanguageParser {
    fn can_handle(&self, file_path: &Path) -> bool;
    fn parse_functions(&mut self, content: &str) -> Result<Vec<FunctionDefinition>>;
    fn parse_functions_ast(&mut self, content: &str) -> Result<Vec<(String, syn::ItemFn)>>;
}

#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub file_path: String,
    // Add more fields as needed
}

pub fn get_parser_for_file(file_path: &Path) -> Option<Box<dyn LanguageParser>> {
    let rust_parser = rust::RustParser::new();
    let python_parser = python::PythonParser::new();

    let parsers: Vec<Box<dyn LanguageParser>> = vec![
        Box::new(rust_parser),
        Box::new(python_parser),
    ];

    parsers.into_iter().find(|p| p.can_handle(file_path))
}
