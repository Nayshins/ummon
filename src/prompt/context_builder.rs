use crate::db::Database;
use crate::graph::entity::EntityType;
use tracing;

pub fn build_context(db: &Database, instruction: &str) -> String {
    let mut context = String::new();
    context.push_str("User Instruction:\n");
    context.push_str(instruction);
    context.push_str("\n\nKnown Functions:\n");

    // Get function entities directly from the database
    let function_entities = match db.query_entities_by_type(&EntityType::Function, None, vec![]) {
        Ok(entities) => entities,
        Err(e) => {
            tracing::error!("Failed to query functions from database: {}", e);
            vec![] // Return empty list on error
        }
    };

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
    let method_entities = match db.query_entities_by_type(&EntityType::Method, None, vec![]) {
        Ok(entities) => entities,
        Err(e) => {
            tracing::error!("Failed to query methods from database: {}", e);
            vec![] // Return empty list on error
        }
    };

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
