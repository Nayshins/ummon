use anyhow::Result;
use std::collections::HashMap;
use tree_sitter::Node;

use crate::graph::entity::EntityId;
use crate::graph::relationship::RelationshipType;
use crate::graph::KnowledgeGraph;

#[allow(dead_code)]
pub fn build_call_graph_for_fn(
    kg: &mut KnowledgeGraph,
    caller_id: &EntityId,
    fn_node: Node,
    code: &str,
    function_map: &HashMap<(String, String), EntityId>,
    file_path: &str,
) -> Result<()> {
    let mut calls = Vec::new();
    collect_calls(fn_node, code, &mut calls);

    for call_name in calls {
        if let Some(callee_id) = function_map.get(&(file_path.to_string(), call_name.clone())) {
            kg.create_relationship(
                caller_id.clone(),
                callee_id.clone(),
                RelationshipType::Calls,
            )?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn collect_calls(node: Node, code: &str, calls: &mut Vec<String>) {
    match node.kind() {
        "call_expression" => {
            if let Some(callee) = node.child_by_field_name("function") {
                let c_name = extract_identifier(callee, code);
                calls.push(c_name);
            }
        }
        "method_call_expression" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let m_name = &code[name_node.start_byte()..name_node.end_byte()];
                calls.push(m_name.to_string());
            }
        }
        _ => {}
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_calls(child, code, calls);
        }
    }
}

#[allow(dead_code)]
fn extract_identifier(node: Node, code: &str) -> String {
    if node.child_count() == 0 {
        return code[node.start_byte()..node.end_byte()].to_string();
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "identifier" {
                return code[child.start_byte()..child.end_byte()].to_string();
            }
        }
    }
    code[node.start_byte()..node.end_byte()].to_string()
}
