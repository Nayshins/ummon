use anyhow::Result;
use std::fs;

use crate::graph::knowledge_graph::KnowledgeGraph;
use crate::parser::language_support::RustParser;
use crate::parser::{call_graph, language_support};

/// Now an async fn, though we don't do real async I/O here—just for consistency
pub async fn run(path: &str) -> Result<()> {
    println!("Indexing code at path: {}", path);

    // 1. Initialize knowledge graph
    let mut kg = KnowledgeGraph::new();

    // 2. Discover code files (naive approach: just .rs)
    let code_files = fs::read_dir(path)?
        .filter_map(|entry| {
            let e = entry.ok()?;
            let p = e.path();
            if p.extension()?.to_str()? == "rs" {
                Some(p)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // 3. Initialize a Rust parser
    let rust_parser = RustParser::new();

    // We'll keep a map: (file_path, fn_name) -> NodeIndex
    let mut fn_map = std::collections::HashMap::new();

    // 4. First pass: parse & add function nodes
    for file in &code_files {
        let file_path_str = file.to_string_lossy().to_string();
        let content = fs::read_to_string(file)?;
        let parsed_funcs = rust_parser.parse_functions(&content)?;

        for fdef in parsed_funcs {
            let idx = kg.add_function_node(fdef.name.clone(), file_path_str.clone());
            fn_map.insert((file_path_str.clone(), fdef.name), idx);
        }
    }

    // 5. Second pass: parse & build call edges
    for file in &code_files {
        let file_path_str = file.to_string_lossy().to_string();
        let content = fs::read_to_string(file)?;
        let func_asts = rust_parser.parse_functions_ast(&content)?;

        for (fn_name, fn_node) in func_asts {
            if let Some(&caller_idx) = fn_map.get(&(file_path_str.clone(), fn_name.clone())) {
                call_graph::build_call_graph_for_fn(
                    &mut kg,
                    caller_idx,
                    fn_node,
                    &content,
                    &fn_map,
                    &file_path_str,
                )?;
            }
        }
    }

    // 6. Save the knowledge graph
    kg.save_to_file("knowledge_graph.json")?;

    println!("Indexing complete. Graph saved to knowledge_graph.json.");
    Ok(())
}
