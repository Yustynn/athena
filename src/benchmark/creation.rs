use crate::error::AthenaError;
use crate::feedback::FragmentFeedback;
use crate::fragment::{Fragment, FragmentKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkSpec {
    pub name: String,
    pub case_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkCaseFile {
    pub family: String,
    pub cases: Vec<CreationBenchmarkCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreationBenchmarkDifficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkCase {
    pub case_id: String,
    pub family: String,
    pub difficulty: CreationBenchmarkDifficulty,
    pub input: CreationBenchmarkInput,
    pub gold: CreationBenchmarkGold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkInput {
    pub purpose: CreationBenchmarkPurpose,
    #[serde(default)]
    pub packet_fragments: Vec<Fragment>,
    #[serde(default)]
    pub fragment_feedback: Vec<FragmentFeedback>,
    #[serde(default)]
    pub outcome_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkPurpose {
    pub statement: String,
    pub success_criteria: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkGold {
    pub should_create: bool,
    pub max_fragments: usize,
    #[serde(default)]
    pub preferred_kind: Option<FragmentKind>,
    #[serde(default)]
    pub required_concepts: Vec<String>,
    #[serde(default)]
    pub forbidden_concepts: Vec<String>,
    #[serde(default)]
    pub concept_aliases: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkProposals {
    pub proposals: Vec<CreationBenchmarkProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkProposal {
    pub case_id: String,
    #[serde(default)]
    pub proposed_fragments: Vec<CreationBenchmarkProposedFragment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationBenchmarkProposedFragment {
    pub kind: FragmentKind,
    pub summary: String,
    pub full_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreationBenchmarkReport {
    pub name: String,
    pub case_results: Vec<CreationBenchmarkCaseResult>,
    pub overall: CreationBenchmarkAggregate,
    pub by_family: BTreeMap<String, CreationBenchmarkAggregate>,
    pub by_difficulty: BTreeMap<String, CreationBenchmarkAggregate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreationBenchmarkCaseResult {
    pub case_id: String,
    pub family: String,
    pub difficulty: CreationBenchmarkDifficulty,
    pub proposed_count: usize,
    pub expected_create: bool,
    pub actual_create: bool,
    pub decision_correct: bool,
    pub count_ok: bool,
    pub kind_match: Option<bool>,
    pub matched_required_concepts: Vec<String>,
    pub missing_required_concepts: Vec<String>,
    pub hit_forbidden_concepts: Vec<String>,
    pub required_recall: f64,
    pub forbidden_clean: bool,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreationBenchmarkAggregate {
    pub case_count: usize,
    pub mean_score: f64,
    pub decision_accuracy: f64,
    pub count_ok_rate: f64,
    pub mean_required_recall: f64,
    pub forbidden_clean_rate: f64,
    pub kind_match_rate: Option<f64>,
}

pub fn run_creation_benchmark(
    spec_path: impl AsRef<Path>,
    proposals_path: impl AsRef<Path>,
) -> Result<CreationBenchmarkReport, AthenaError> {
    let spec_path = spec_path.as_ref();
    let spec: CreationBenchmarkSpec = read_json(spec_path)?;
    let base_dir = spec_path
        .parent()
        .ok_or_else(|| std::io::Error::other("benchmark spec path missing parent"))?;
    let cases = load_cases(base_dir, &spec.case_files)?;
    let proposals: CreationBenchmarkProposals = read_json(proposals_path.as_ref())?;
    let proposal_map = proposals
        .proposals
        .into_iter()
        .map(|proposal| (proposal.case_id.clone(), proposal))
        .collect::<BTreeMap<_, _>>();

    let case_results = cases
        .iter()
        .map(|case| {
            let proposal = proposal_map.get(&case.case_id);
            score_case(case, proposal)
        })
        .collect::<Vec<_>>();

    Ok(CreationBenchmarkReport {
        name: spec.name,
        overall: aggregate(&case_results),
        by_family: aggregate_by(&case_results, |result| result.family.clone()),
        by_difficulty: aggregate_by(&case_results, |result| difficulty_key(&result.difficulty)),
        case_results,
    })
}

fn load_cases(
    base_dir: &Path,
    paths: &[String],
) -> Result<Vec<CreationBenchmarkCase>, AthenaError> {
    let mut cases = Vec::new();
    for relative_path in paths {
        let path = base_dir.join(relative_path);
        let case_file: CreationBenchmarkCaseFile = read_json(&path)?;
        cases.extend(case_file.cases);
    }
    Ok(cases)
}

fn score_case(
    case: &CreationBenchmarkCase,
    proposal: Option<&CreationBenchmarkProposal>,
) -> CreationBenchmarkCaseResult {
    let proposed_fragments = proposal
        .map(|proposal| proposal.proposed_fragments.clone())
        .unwrap_or_default();
    let proposed_count = proposed_fragments.len();
    let actual_create = proposed_count > 0;
    let expected_create = case.gold.should_create;
    let decision_correct = actual_create == expected_create;
    let count_ok = if expected_create {
        proposed_count > 0 && proposed_count <= case.gold.max_fragments
    } else {
        proposed_count == 0
    };
    let kind_match = case.gold.preferred_kind.as_ref().map(|kind| {
        proposed_fragments
            .iter()
            .any(|fragment| fragment.kind == *kind)
    });
    let proposed_text = proposed_fragments
        .iter()
        .map(|fragment| format!("{} {}", fragment.summary, fragment.full_text))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    let matched_required_concepts = case
        .gold
        .required_concepts
        .iter()
        .filter(|concept| concept_matches(&proposed_text, &case.gold.concept_aliases, concept))
        .cloned()
        .collect::<Vec<_>>();
    let missing_required_concepts = case
        .gold
        .required_concepts
        .iter()
        .filter(|concept| !matched_required_concepts.contains(concept))
        .cloned()
        .collect::<Vec<_>>();
    let hit_forbidden_concepts = case
        .gold
        .forbidden_concepts
        .iter()
        .filter(|concept| concept_matches(&proposed_text, &case.gold.concept_aliases, concept))
        .cloned()
        .collect::<Vec<_>>();

    let required_recall = if case.gold.required_concepts.is_empty() {
        1.0
    } else {
        matched_required_concepts.len() as f64 / case.gold.required_concepts.len() as f64
    };
    let forbidden_clean = hit_forbidden_concepts.is_empty();
    let kind_score = kind_match.map(bool_to_f64).unwrap_or(1.0);
    let score = mean(
        [
            bool_to_f64(decision_correct),
            bool_to_f64(count_ok),
            kind_score,
            required_recall,
            bool_to_f64(forbidden_clean),
        ]
        .into_iter(),
    );

    CreationBenchmarkCaseResult {
        case_id: case.case_id.clone(),
        family: case.family.clone(),
        difficulty: case.difficulty.clone(),
        proposed_count,
        expected_create,
        actual_create,
        decision_correct,
        count_ok,
        kind_match,
        matched_required_concepts,
        missing_required_concepts,
        hit_forbidden_concepts,
        required_recall,
        forbidden_clean,
        score,
    }
}

fn concept_matches(text: &str, aliases: &BTreeMap<String, Vec<String>>, concept: &str) -> bool {
    aliases
        .get(concept)
        .cloned()
        .unwrap_or_else(|| vec![concept.to_owned()])
        .into_iter()
        .map(|alias| alias.to_lowercase())
        .any(|alias| text.contains(&alias))
}

fn aggregate(results: &[CreationBenchmarkCaseResult]) -> CreationBenchmarkAggregate {
    let case_count = results.len();
    let kind_matches = results
        .iter()
        .filter_map(|result| result.kind_match.map(bool_to_f64))
        .collect::<Vec<_>>();

    CreationBenchmarkAggregate {
        case_count,
        mean_score: mean(results.iter().map(|result| result.score)),
        decision_accuracy: mean(
            results
                .iter()
                .map(|result| bool_to_f64(result.decision_correct)),
        ),
        count_ok_rate: mean(results.iter().map(|result| bool_to_f64(result.count_ok))),
        mean_required_recall: mean(results.iter().map(|result| result.required_recall)),
        forbidden_clean_rate: mean(
            results
                .iter()
                .map(|result| bool_to_f64(result.forbidden_clean)),
        ),
        kind_match_rate: if kind_matches.is_empty() {
            None
        } else {
            Some(mean(kind_matches.into_iter()))
        },
    }
}

fn aggregate_by(
    results: &[CreationBenchmarkCaseResult],
    key_fn: impl Fn(&CreationBenchmarkCaseResult) -> String,
) -> BTreeMap<String, CreationBenchmarkAggregate> {
    let mut grouped = BTreeMap::<String, Vec<CreationBenchmarkCaseResult>>::new();
    for result in results {
        grouped
            .entry(key_fn(result))
            .or_default()
            .push(result.clone());
    }

    grouped
        .into_iter()
        .map(|(key, group)| (key, aggregate(&group)))
        .collect()
}

fn difficulty_key(difficulty: &CreationBenchmarkDifficulty) -> String {
    match difficulty {
        CreationBenchmarkDifficulty::Easy => "easy".into(),
        CreationBenchmarkDifficulty::Medium => "medium".into(),
        CreationBenchmarkDifficulty::Hard => "hard".into(),
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
