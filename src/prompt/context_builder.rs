use crate::graph::KnowledgeGraph;

pub fn build_context(kg: &KnowledgeGraph, instruction: &str) -> String {
    let mut context = String::new();
    context.push_str("User Instruction:\n");
    context.push_str(instruction);
    context.push_str("\n\nKnown Functions:\n");

    for (_key, func) in kg.get_functions() {
        context.push_str("- ");
        context.push_str(&format!("{}::{}", func.file_path, func.name));
        context.push('\n');
    }

    context
}
