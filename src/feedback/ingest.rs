use crate::feedback::{FeedbackEvent, FragmentVerdict};
use std::collections::BTreeMap;

pub type FragmentScores = BTreeMap<String, i32>;

pub fn apply_feedback(scores: &mut FragmentScores, feedback: &FeedbackEvent) {
    for item in &feedback.fragment_feedback {
        let delta = match item.verdict {
            FragmentVerdict::Helped => 1,
            FragmentVerdict::Neutral => 0,
            FragmentVerdict::Wrong => -2,
        };

        let entry = scores.entry(item.fragment_id.0.clone()).or_insert(0);
        *entry += delta;
    }
}
