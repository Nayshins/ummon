use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

use crate::graph::entity::{Entity, EntityType};
use crate::graph::KnowledgeGraph;

/// Find entities by a search pattern
#[allow(dead_code)]
pub fn find_entities_by_pattern<'a>(
    kg: &'a KnowledgeGraph,
    pattern: &str,
    entity_types: Option<&[EntityType]>,
) -> Vec<&'a dyn Entity> {
    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => {
            // If regex fails, do a simple string match
            return kg.get_all_entities()
                .into_iter()
                .filter(|e| {
                    // Check entity types if specified
                    if let Some(types) = entity_types {
                        if !types.contains(&e.entity_type()) {
                            return false;
                        }
                    }
                    
                    e.name().contains(pattern)
                })
                .collect();
        }
    };
    
    kg.get_all_entities()
        .into_iter()
        .filter(|e| {
            // Check entity types if specified
            if let Some(types) = entity_types {
                if !types.contains(&e.entity_type()) {
                    return false;
                }
            }
            
            regex.is_match(e.name())
        })
        .collect()
}

/// Extract a file name from a path
#[allow(dead_code)]
pub fn extract_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

/// Extract a module name from a file path
#[allow(dead_code)]
pub fn extract_module_name(path: &str) -> String {
    let file_name = extract_file_name(path);
    
    Path::new(&file_name)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(&file_name)
        .to_string()
}

/// Get a list of resource paths for MCP
#[allow(dead_code)]
pub fn get_mcp_resources(
    kg: &KnowledgeGraph,
    resource_type: &str,
    pattern: Option<&str>,
) -> Vec<String> {
    let all_entities = kg.get_all_entities();
    
    match resource_type {
        "files" => {
            let mut files = Vec::new();
            for entity in all_entities {
                if let EntityType::File | EntityType::Module = entity.entity_type() {
                    if let Some(path) = entity.metadata().get("path").or_else(|| entity.metadata().get("file_path")) {
                        if let Some(p) = pattern {
                            if path.contains(p) {
                                files.push(path.clone());
                            }
                        } else {
                            files.push(path.clone());
                        }
                    }
                }
            }
            files
        },
        "functions" => {
            let mut functions = Vec::new();
            for entity in all_entities {
                if let EntityType::Function | EntityType::Method = entity.entity_type() {
                    let name = entity.name();
                    if let Some(p) = pattern {
                        if name.contains(p) {
                            functions.push(name.to_string());
                        }
                    } else {
                        functions.push(name.to_string());
                    }
                }
            }
            functions
        },
        "classes" => {
            let mut classes = Vec::new();
            for entity in all_entities {
                if let EntityType::Class | EntityType::Struct | EntityType::Trait | EntityType::Interface = entity.entity_type() {
                    let name = entity.name();
                    if let Some(p) = pattern {
                        if name.contains(p) {
                            classes.push(name.to_string());
                        }
                    } else {
                        classes.push(name.to_string());
                    }
                }
            }
            classes
        },
        "domains" => {
            kg.get_domain_concepts()
                .into_iter()
                .filter(|c| {
                    if let Some(p) = pattern {
                        c.name().contains(p)
                    } else {
                        true
                    }
                })
                .map(|c| c.name().to_string())
                .collect()
        },
        _ => Vec::new(),
    }
}

/// Format entity data for API response
#[allow(dead_code)]
pub fn format_entity_for_response(entity: &dyn Entity) -> HashMap<String, serde_json::Value> {
    let mut data = HashMap::new();
    
    data.insert("id".to_string(), serde_json::json!(entity.id().as_str()));
    data.insert("name".to_string(), serde_json::json!(entity.name()));
    data.insert("type".to_string(), serde_json::json!(format!("{:?}", entity.entity_type())));
    
    if let Some(loc) = entity.location() {
        data.insert("location".to_string(), serde_json::json!({
            "start": {
                "line": loc.start.line,
                "column": loc.start.column,
                "offset": loc.start.offset,
            },
            "end": {
                "line": loc.end.line,
                "column": loc.end.column,
                "offset": loc.end.offset,
            },
        }));
    }
    
    // Add metadata
    let metadata = entity.metadata()
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::json!(v)))
        .collect::<HashMap<_, _>>();
        
    data.insert("metadata".to_string(), serde_json::json!(metadata));
    
    data
}
