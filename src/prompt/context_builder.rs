use crate::graph::entity::EntityType;
use crate::graph::KnowledgeGraph;

pub fn build_context(kg: &KnowledgeGraph, instruction: &str) -> String {
    let mut context = String::new();
    context.push_str("User Instruction:\n");
    context.push_str(instruction);
    context.push_str("\n\nKnown Functions:\n");

    // Get function entities directly from the entity model
    let function_entities = kg.get_entities_by_type(&EntityType::Function);
    for func in function_entities {
        context.push_str("- ");
        if let Some(file_path) = func.file_path() {
            context.push_str(&format!("{}::{}", file_path, func.name()));
        } else {
            context.push_str(func.name());
        }
        context.push('\n');
    }

    // Also include method entities
    let method_entities = kg.get_entities_by_type(&EntityType::Method);
    for method in method_entities {
        context.push_str("- ");
        if let Some(file_path) = method.file_path() {
            context.push_str(&format!("{}::{}", file_path, method.name()));
        } else {
            context.push_str(method.name());
        }
        context.push('\n');
    }

    context
}
