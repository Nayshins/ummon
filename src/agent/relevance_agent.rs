use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing;

use crate::db::Database;
use crate::graph::entity::{Entity, EntityId, EntityType};
use crate::graph::relationship::RelationshipType;
use crate::prompt::llm_integration::{get_llm_config, query_llm};

/// Represents a file with relevance to a proposed change
#[derive(Debug, Serialize, Deserialize)]
pub struct RelevantFile {
    pub path: String,
    pub relevance_score: f32,
    pub contributing_entity_ids: Vec<EntityId>,
}

/// Suggests relevant files based on a proposed change
pub async fn suggest_relevant_files(change: &str, db: &Database) -> Result<Vec<RelevantFile>> {
    let keywords = extract_keywords(change).await?;
    tracing::info!("Extracted keywords: {:?}", keywords);

    let seed_entities = search_seed_entities(db, &keywords)?;
    tracing::info!("Found {} seed entities", seed_entities.len());

    let expanded_entities = expand_context(db, &seed_entities)?;
    tracing::info!("Expanded to {} entities", expanded_entities.len());

    let ranked_entities = rank_entities(db, expanded_entities)?;
    tracing::info!("Ranked {} entities", ranked_entities.len());

    let ranked_files = aggregate_and_rank_files(ranked_entities)?;
    tracing::info!("Ranked {} files", ranked_files.len());

    Ok(ranked_files)
}

/// Extract technical keywords from the proposed change using LLM
async fn extract_keywords(change: &str) -> Result<Vec<String>> {
    let llm_config = get_llm_config(None, None);
    let prompt = format!(
        r#"Analyze the following proposed change and extract key technical concepts, entity names, domain terms, and actions as a JSON array of strings.

Input: "{}"

Example query: "Add authentication to the login system"
Example output: ["authentication", "login system", "user credentials", "session management"]

Example query: "Fix bug in database connection pooling"
Example output: ["database connection", "connection pooling", "bug fix", "resource management"]

Example query: "Implement file relevance scoring algorithm"
Example output: ["relevance scoring", "algorithm", "file ranking", "search"]

The response MUST be a valid JSON array containing ONLY strings.
Return ONLY the JSON array without any explanation, markdown formatting, or other text."#,
        change
    );

    let response = query_llm(&prompt, &llm_config).await?;
    let cleaned_response = response.trim().trim_matches(|c| c == '`' || c == '"');

    match serde_json::from_str::<Vec<String>>(cleaned_response) {
        Ok(keywords) => Ok(keywords),
        Err(e) => {
            tracing::warn!("Failed to parse keywords from LLM response: {}", e);
            let fallback_keywords = extract_keywords_fallback(cleaned_response);
            if !fallback_keywords.is_empty() {
                Ok(fallback_keywords)
            } else {
                let basic_keywords = change
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>();
                Ok(basic_keywords)
            }
        }
    }
}

/// Fallback method to extract keywords from LLM response when JSON parsing fails
fn extract_keywords_fallback(response: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    let cleaned = response
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    for line in cleaned.lines() {
        let line = line.trim();

        if let Some(quoted) = line
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim_end_matches(',')
            .trim()
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
        {
            keywords.push(quoted.to_string());
        }
    }

    keywords
}

/// Search for seed entities matching the extracted keywords
fn search_seed_entities(db: &Database, keywords: &[String]) -> Result<Vec<(Box<dyn Entity>, f32)>> {
    let mut seed_entities = Vec::new();
    let entity_types = vec![
        EntityType::Function,
        EntityType::Method,
        EntityType::Class,
        EntityType::Module,
        EntityType::Variable,
        EntityType::Constant,
        EntityType::DomainConcept,
    ];

    for entity_type in entity_types {
        let conditions: Vec<String> = keywords
            .iter()
            .flat_map(|kw| {
                vec![
                    format!("name LIKE '%{}%'", kw.replace('\'', "''")),
                    format!("file_path LIKE '%{}%'", kw.replace('\'', "''")),
                    format!("documentation LIKE '%{}%'", kw.replace('\'', "''")),
                ]
            })
            .collect();

        if conditions.is_empty() {
            continue;
        }

        let condition = conditions.join(" OR ");
        let entities = db.query_entities_by_type(&entity_type, Some(&condition))?;

        for entity in entities {
            let mut score = 0.0;
            let entity_str = format!(
                "{} {} {}",
                entity.name(),
                entity.file_path().unwrap_or(&String::new()),
                entity
                    .metadata()
                    .get("documentation")
                    .unwrap_or(&String::new())
            )
            .to_lowercase();

            for kw in keywords {
                if entity_str.contains(&kw.to_lowercase()) {
                    score += 1.0;

                    if entity.name().to_lowercase().contains(&kw.to_lowercase()) {
                        score += 2.0;
                    }
                }
            }

            if score > 0.0 {
                seed_entities.push((entity, score));
            }
        }
    }

    Ok(seed_entities)
}

/// Expand context via relationship traversal
fn expand_context(
    db: &Database,
    seed_entities: &[(Box<dyn Entity>, f32)],
) -> Result<Vec<(Box<dyn Entity>, f32)>> {
    let mut all_entities: Vec<(Box<dyn Entity>, f32)> = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let relationship_types = vec![
        RelationshipType::Calls,
        RelationshipType::Contains,
        RelationshipType::Imports,
        RelationshipType::References,
        RelationshipType::RepresentedBy,
    ];

    let max_depth = 2;

    for (entity, score) in seed_entities {
        if let Some(loaded_entity) = db.load_entity(entity.id())? {
            all_entities.push((loaded_entity, *score));
            seen_ids.insert(entity.id().clone());
        }
    }

    for (seed_entity, seed_score) in seed_entities {
        for rel_type in &relationship_types {
            let paths = db.find_paths(
                seed_entity.id(),
                None,
                None,
                Some(rel_type),
                max_depth,
                "both",
            )?;

            for (entity_id, depth) in paths {
                if depth > 0 && !seen_ids.contains(&entity_id) {
                    seen_ids.insert(entity_id.clone());

                    if let Some(entity) = db.load_entity(&entity_id)? {
                        let proximity_score = seed_score * (1.0 / (depth as f32 + 1.0));
                        all_entities.push((entity, proximity_score));
                    }
                }
            }
        }
    }

    Ok(all_entities)
}

/// Rank entities using a hybrid scoring approach
fn rank_entities(
    db: &Database,
    entities: Vec<(Box<dyn Entity>, f32)>,
) -> Result<Vec<(Box<dyn Entity>, f32)>> {
    let mut ranked_entities = Vec::new();
    let entity_ids: Vec<EntityId> = entities.iter().map(|(e, _)| e.id().clone()).collect();

    let mut centrality_scores = std::collections::HashMap::new();
    for entity_id in &entity_ids {
        let rels = db.load_relationships_for_entity(entity_id)?;
        let degree = rels
            .iter()
            .filter(|r| entity_ids.contains(&r.source_id) || entity_ids.contains(&r.target_id))
            .count() as f32;
        centrality_scores.insert(entity_id.clone(), degree);
    }

    let max_centrality = centrality_scores.values().cloned().fold(0.0, f32::max);
    let normalized_centrality: std::collections::HashMap<_, _> = centrality_scores
        .into_iter()
        .map(|(id, score)| {
            (
                id,
                if max_centrality > 0.0 {
                    score / max_centrality
                } else {
                    0.0
                },
            )
        })
        .collect();

    for (entity, proximity_score) in entities {
        let centrality = normalized_centrality.get(entity.id()).unwrap_or(&0.0);
        let final_score = proximity_score * 0.7 + centrality * 0.3;
        ranked_entities.push((entity, final_score));
    }

    ranked_entities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(ranked_entities)
}

/// Aggregate entity scores into file-level scores
fn aggregate_and_rank_files(entities: Vec<(Box<dyn Entity>, f32)>) -> Result<Vec<RelevantFile>> {
    let mut file_map: std::collections::HashMap<String, (f32, Vec<EntityId>)> =
        std::collections::HashMap::new();

    for (entity, score) in entities {
        if let Some(file_path) = entity.file_path() {
            let entry = file_map
                .entry(file_path.to_string())
                .or_insert((0.0, Vec::new()));

            entry.0 = entry.0.max(score);
            entry.1.push(entity.id().clone());
        }
    }

    let mut files: Vec<RelevantFile> = file_map
        .into_iter()
        .map(|(path, (score, entity_ids))| RelevantFile {
            path,
            relevance_score: score,
            contributing_entity_ids: entity_ids,
        })
        .collect();

    files.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if files.len() > 10 {
        files.truncate(10);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity::{BaseEntity, EntityType};

    #[test]
    fn test_extract_keywords_fallback() {
        let input1 = r#"["keyword1", "keyword2", "keyword3"]"#;
        let keywords1 = extract_keywords_fallback(input1);
        assert!(!keywords1.is_empty());

        let input2 = r#"
        [
          "function",
          "authentication",
          "login"
        ]
        "#;
        let keywords2 = extract_keywords_fallback(input2);
        assert!(!keywords2.is_empty());
    }

    #[test]
    fn test_aggregate_and_rank_files() {
        let id1 = EntityId::new("test1");
        let base1 = BaseEntity::new(
            id1,
            "test1".to_string(),
            EntityType::Function,
            Some("file1.rs".to_string()),
        );

        let id2 = EntityId::new("test2");
        let base2 = BaseEntity::new(
            id2,
            "test2".to_string(),
            EntityType::Function,
            Some("file1.rs".to_string()),
        );

        let id3 = EntityId::new("test3");
        let base3 = BaseEntity::new(
            id3,
            "test3".to_string(),
            EntityType::Function,
            Some("file2.rs".to_string()),
        );

        let entities = vec![
            (Box::new(base1) as Box<dyn Entity>, 0.8),
            (Box::new(base2) as Box<dyn Entity>, 0.5),
            (Box::new(base3) as Box<dyn Entity>, 0.6),
        ];

        let files = aggregate_and_rank_files(entities).unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "file1.rs");
        assert_eq!(files[0].relevance_score, 0.8);
        assert_eq!(files[0].contributing_entity_ids.len(), 2);
        assert_eq!(files[1].path, "file2.rs");
        assert_eq!(files[1].relevance_score, 0.6);
        assert_eq!(files[1].contributing_entity_ids.len(), 1);
    }
}
