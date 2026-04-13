use crate::error::AthenaError;
use crate::fragment::{Fragment, load_fragments};
use crate::ids::PurposeId;
use crate::packet::rank_fragments;
use crate::purpose::{Purpose, PurposeStatus};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalBenchmarkSpec {
    pub name: String,
    pub k_values: Vec<usize>,
    pub corpuses: Vec<String>,
    pub task_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalBenchmarkCorpus {
    pub corpus_id: String,
    pub fragments: Vec<Fragment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalTaskFile {
    pub family: String,
    pub tasks: Vec<RetrievalTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalTask {
    pub task_id: String,
    pub family: String,
    pub difficulty: RetrievalDifficulty,
    pub corpus_id: String,
    pub prompt: String,
    pub success_criteria: String,
    pub gold: RetrievalGold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalGold {
    pub required_matches: Vec<String>,
    #[serde(default)]
    pub preferred_order: Vec<String>,
    #[serde(default)]
    pub acceptable_sets: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalBenchmarkReport {
    pub name: String,
    pub k_values: Vec<usize>,
    pub corpuses: Vec<String>,
    pub task_results: Vec<RetrievalTaskResult>,
    pub overall: RetrievalBenchmarkAggregate,
    pub by_corpus: BTreeMap<String, RetrievalBenchmarkAggregate>,
    pub by_family: BTreeMap<String, RetrievalBenchmarkAggregate>,
    pub by_difficulty: BTreeMap<String, RetrievalBenchmarkAggregate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalTaskResult {
    pub task_id: String,
    pub family: String,
    pub difficulty: RetrievalDifficulty,
    pub corpus_id: String,
    pub ranked_fragment_ids: Vec<String>,
    pub relevant_fragment_ids: Vec<String>,
    pub required_ranks: BTreeMap<String, Option<usize>>,
    pub hit_at: BTreeMap<String, bool>,
    pub coverage_at: BTreeMap<String, f64>,
    pub mrr: f64,
    pub ndcg_at: BTreeMap<String, f64>,
    pub first_relevant_rank: Option<usize>,
    pub preferred_order_satisfied: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalBenchmarkAggregate {
    pub task_count: usize,
    pub hit_rate_at: BTreeMap<String, f64>,
    pub mean_coverage_at: BTreeMap<String, f64>,
    pub mean_ndcg_at: BTreeMap<String, f64>,
    pub mean_mrr: f64,
    pub mean_first_relevant_rank: Option<f64>,
}

pub fn run_retrieval_benchmark(
    spec_path: impl AsRef<Path>,
) -> Result<RetrievalBenchmarkReport, AthenaError> {
    let spec_path = spec_path.as_ref();
    let spec: RetrievalBenchmarkSpec = read_json(spec_path)?;
    let base_dir = spec_path
        .parent()
        .ok_or_else(|| std::io::Error::other("benchmark spec path missing parent"))?;

    let corpuses = load_corpuses(base_dir, &spec.corpuses)?;
    let tasks = load_tasks(base_dir, &spec.task_files)?;
    let max_k = spec.k_values.iter().copied().max().unwrap_or(5);

    let task_results = tasks
        .iter()
        .map(|task| run_task(task, corpuses.get(&task.corpus_id), &spec.k_values, max_k))
        .collect::<Result<Vec<_>, AthenaError>>()?;

    Ok(RetrievalBenchmarkReport {
        name: spec.name,
        k_values: spec.k_values.clone(),
        corpuses: corpuses.keys().cloned().collect(),
        overall: aggregate(&task_results, &spec.k_values),
        by_corpus: aggregate_by(
            &task_results,
            &spec.k_values,
            |result| result.corpus_id.clone(),
        ),
        by_family: aggregate_by(
            &task_results,
            &spec.k_values,
            |result| result.family.clone(),
        ),
        by_difficulty: aggregate_by(
            &task_results,
            &spec.k_values,
            |result| difficulty_key(&result.difficulty),
        ),
        task_results,
    })
}

fn load_corpuses(
    base_dir: &Path,
    paths: &[String],
) -> Result<BTreeMap<String, Vec<Fragment>>, AthenaError> {
    let mut corpuses = BTreeMap::new();
    for relative_path in paths {
        let path = base_dir.join(relative_path);
        let corpus: RetrievalBenchmarkCorpus = read_json(&path)?;
        corpuses.insert(corpus.corpus_id, corpus.fragments);
    }
    Ok(corpuses)
}

fn load_tasks(base_dir: &Path, paths: &[String]) -> Result<Vec<RetrievalTask>, AthenaError> {
    let mut tasks = Vec::new();
    for relative_path in paths {
        let path = base_dir.join(relative_path);
        let task_file: RetrievalTaskFile = read_json(&path)?;
        tasks.extend(task_file.tasks);
    }
    Ok(tasks)
}

fn run_task(
    task: &RetrievalTask,
    fragments: Option<&Vec<Fragment>>,
    k_values: &[usize],
    max_k: usize,
) -> Result<RetrievalTaskResult, AthenaError> {
    let fragments = fragments.ok_or_else(|| {
        AthenaError::Io(std::io::Error::other(format!(
            "missing corpus for task {}: {}",
            task.task_id, task.corpus_id
        )))
    })?;
    let purpose = Purpose {
        purpose_id: PurposeId::new(format!("purpose-{}", task.task_id)),
        statement: task.prompt.clone(),
        success_criteria: task.success_criteria.clone(),
        status: PurposeStatus::Active,
    };
    let ranked = rank_fragments(&purpose, fragments);
    let ranked_fragment_ids = ranked
        .iter()
        .take(max_k)
        .map(|fragment| fragment.fragment_id.0.clone())
        .collect::<Vec<_>>();
    let relevant_fragment_ids = relevant_ids(&task.gold).into_iter().collect::<Vec<_>>();
    let rank_lookup = ranked
        .iter()
        .enumerate()
        .map(|(idx, fragment)| (fragment.fragment_id.0.clone(), idx + 1))
        .collect::<BTreeMap<_, _>>();

    let required_ranks = task
        .gold
        .required_matches
        .iter()
        .map(|fragment_id| (fragment_id.clone(), rank_lookup.get(fragment_id).copied()))
        .collect::<BTreeMap<_, _>>();

    let hit_at = k_values
        .iter()
        .map(|k| (k.to_string(), hit_at_k(&ranked_fragment_ids, &task.gold, *k)))
        .collect::<BTreeMap<_, _>>();

    let coverage_at = k_values
        .iter()
        .map(|k| {
            (
                k.to_string(),
                coverage_at_k(&ranked_fragment_ids, &task.gold, *k),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let ndcg_at = k_values
        .iter()
        .map(|k| (k.to_string(), ndcg_at_k(&ranked_fragment_ids, &task.gold, *k)))
        .collect::<BTreeMap<_, _>>();

    Ok(RetrievalTaskResult {
        task_id: task.task_id.clone(),
        family: task.family.clone(),
        difficulty: task.difficulty.clone(),
        corpus_id: task.corpus_id.clone(),
        ranked_fragment_ids,
        relevant_fragment_ids,
        required_ranks,
        hit_at,
        coverage_at,
        mrr: reciprocal_rank(&ranked, &task.gold),
        ndcg_at,
        first_relevant_rank: first_relevant_rank(&ranked, &task.gold),
        preferred_order_satisfied: preferred_order_satisfied(&rank_lookup, &task.gold),
    })
}

fn aggregate(
    results: &[RetrievalTaskResult],
    k_values: &[usize],
) -> RetrievalBenchmarkAggregate {
    let task_count = results.len();
    let hit_rate_at = k_values
        .iter()
        .map(|k| {
            let key = k.to_string();
            let value = mean(results.iter().map(|result| bool_to_f64(result.hit_at[&key])));
            (key, value)
        })
        .collect();
    let mean_coverage_at = k_values
        .iter()
        .map(|k| {
            let key = k.to_string();
            let value = mean(results.iter().map(|result| result.coverage_at[&key]));
            (key, value)
        })
        .collect();
    let mean_ndcg_at = k_values
        .iter()
        .map(|k| {
            let key = k.to_string();
            let value = mean(results.iter().map(|result| result.ndcg_at[&key]));
            (key, value)
        })
        .collect();
    let first_relevant_ranks = results
        .iter()
        .filter_map(|result| result.first_relevant_rank.map(|rank| rank as f64))
        .collect::<Vec<_>>();

    RetrievalBenchmarkAggregate {
        task_count,
        hit_rate_at,
        mean_coverage_at,
        mean_ndcg_at,
        mean_mrr: mean(results.iter().map(|result| result.mrr)),
        mean_first_relevant_rank: if first_relevant_ranks.is_empty() {
            None
        } else {
            Some(mean(first_relevant_ranks.into_iter()))
        },
    }
}

fn aggregate_by(
    results: &[RetrievalTaskResult],
    k_values: &[usize],
    key_fn: impl Fn(&RetrievalTaskResult) -> String,
) -> BTreeMap<String, RetrievalBenchmarkAggregate> {
    let mut grouped = BTreeMap::<String, Vec<RetrievalTaskResult>>::new();
    for result in results {
        grouped
            .entry(key_fn(result))
            .or_default()
            .push(result.clone());
    }

    grouped
        .into_iter()
        .map(|(key, group)| (key, aggregate(&group, k_values)))
        .collect()
}

fn hit_at_k(ranked: &[String], gold: &RetrievalGold, k: usize) -> bool {
    let relevant = relevant_ids(gold);
    ranked
        .iter()
        .take(k)
        .any(|fragment_id| relevant.contains(fragment_id))
}

fn coverage_at_k(ranked: &[String], gold: &RetrievalGold, k: usize) -> f64 {
    let top = ranked.iter().take(k).cloned().collect::<BTreeSet<_>>();
    let mut units = gold.required_matches.len() + gold.acceptable_sets.len();
    if units == 0 {
        units = 1;
    }

    let mut satisfied = gold
        .required_matches
        .iter()
        .filter(|fragment_id| top.contains(*fragment_id))
        .count();
    satisfied += gold
        .acceptable_sets
        .iter()
        .filter(|set| set.iter().any(|fragment_id| top.contains(fragment_id)))
        .count();

    satisfied as f64 / units as f64
}

fn reciprocal_rank(ranked: &[Fragment], gold: &RetrievalGold) -> f64 {
    first_relevant_rank(ranked, gold)
        .map(|rank| 1.0 / rank as f64)
        .unwrap_or(0.0)
}

fn first_relevant_rank(ranked: &[Fragment], gold: &RetrievalGold) -> Option<usize> {
    let relevant = relevant_ids(gold);
    ranked
        .iter()
        .position(|fragment| relevant.contains(&fragment.fragment_id.0))
        .map(|idx| idx + 1)
}

fn ndcg_at_k(ranked: &[String], gold: &RetrievalGold, k: usize) -> f64 {
    let relevant = relevant_ids(gold);
    if relevant.is_empty() {
        return 0.0;
    }

    let dcg = ranked
        .iter()
        .take(k)
        .enumerate()
        .filter(|(_, fragment_id)| relevant.contains(*fragment_id))
        .map(|(idx, _)| 1.0 / ((idx + 2) as f64).log2())
        .sum::<f64>();

    let ideal_len = relevant.len().min(k);
    let ideal = (0..ideal_len)
        .map(|idx| 1.0 / ((idx + 2) as f64).log2())
        .sum::<f64>();

    if ideal == 0.0 { 0.0 } else { dcg / ideal }
}

fn preferred_order_satisfied(
    rank_lookup: &BTreeMap<String, usize>,
    gold: &RetrievalGold,
) -> Option<bool> {
    if gold.preferred_order.len() < 2 {
        return None;
    }

    let mut previous = None;
    let mut saw_any = false;

    for fragment_id in &gold.preferred_order {
        let Some(rank) = rank_lookup.get(fragment_id).copied() else {
            continue;
        };
        saw_any = true;
        if let Some(previous_rank) = previous {
            if rank < previous_rank {
                return Some(false);
            }
        }
        previous = Some(rank);
    }

    if saw_any { Some(true) } else { None }
}

fn relevant_ids(gold: &RetrievalGold) -> BTreeSet<String> {
    let mut ids = gold.required_matches.iter().cloned().collect::<BTreeSet<_>>();
    for set in &gold.acceptable_sets {
        ids.extend(set.iter().cloned());
    }
    ids
}

fn difficulty_key(difficulty: &RetrievalDifficulty) -> String {
    match difficulty {
        RetrievalDifficulty::Easy => "easy".into(),
        RetrievalDifficulty::Medium => "medium".into(),
        RetrievalDifficulty::Hard => "hard".into(),
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for value in values {
        sum += value;
        count += 1.0;
    }
    if count == 0.0 { 0.0 } else { sum / count }
}

fn bool_to_f64(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, AthenaError> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(AthenaError::from)
}

#[allow(dead_code)]
fn _read_fragment_fixture(path: &PathBuf) -> Result<Vec<Fragment>, AthenaError> {
    load_fragments(path)
}
