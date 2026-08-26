//! 写作时的上下文浮带：按当前段落多信号匹配预先设计的人物 / 设定 / 伏笔。

mod entry_match;

pub use entry_match::{match_story_entry, EntryMatch};

use novel_domain::{
    CanonEntity, CanonFact, ContextHint, EntityKind, HintAction, HintKind, PlotThread,
    PlotThreadStatus, Revision, StoryEntry, StoryEntryKind, WorkContextRef,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HintQuery {
    pub work_ref: WorkContextRef,
    pub nearby_text: String,
    #[serde(default)]
    pub lookback_text: String,
    pub generation: u64,
    pub limit: usize,
}

pub struct HintEngine {
    pub minimum_dwell_score: f32,
}

impl HintEngine {
    pub fn rank(
        &self,
        query: &HintQuery,
        entities: &[CanonEntity],
        facts: &[CanonFact],
        threads: &[PlotThread],
    ) -> Vec<ContextHint> {
        let mut hints = Vec::new();

        for entity in entities {
            let matched_name = entity.canonical_name.clone();
            let score = name_score(query, entity);
            if score <= 0.0 {
                continue;
            }
            let (kind, source, summary) = match entity.kind {
                EntityKind::Character => (
                    HintKind::CharacterState,
                    "人物",
                    format!("{} · 预先设定的人物", entity.canonical_name),
                ),
                _ => (
                    HintKind::WorldRule,
                    "设定",
                    format!("{} · 预先设定的条目", entity.canonical_name),
                ),
            };
            hints.push(hint(
                kind,
                &matched_name,
                &summary,
                source,
                "当前段落出现该名称",
                score,
                query,
            ));
        }

        for fact in facts {
            let text = match &fact.value {
                serde_json::Value::String(value) => value.clone(),
                other => other.to_string(),
            };
            if fact.predicate == "note" {
                continue;
            }
            if fact_relevant(query, &fact.predicate, &text) {
                hints.push(hint(
                    HintKind::WorldRule,
                    &fact.predicate,
                    &format!("{}: {}", fact.predicate, text),
                    "设定",
                    "当前文字与设定匹配",
                    fact.confidence,
                    query,
                ));
            }
        }

        for thread in threads {
            if thread.status != PlotThreadStatus::Open {
                continue;
            }
            if !query.nearby_text.contains(&thread.title) {
                continue;
            }
            let summary = if thread.summary.is_empty() {
                format!("伏笔「{}」仍未兑现", thread.title)
            } else {
                thread.summary.clone()
            };
            hints.push(hint(
                HintKind::OpenForeshadowing,
                &thread.title,
                &summary,
                "伏笔",
                "当前段落提到该伏笔",
                0.86,
                query,
            ));
        }

        hints.sort_by(|left, right| right.score.total_cmp(&left.score));
        hints.truncate(query.limit.clamp(1, 6));
        hints
    }

    /// 按当前段落匹配作者预先写好的人物 / 设定 / 伏笔。
    pub fn rank_entries(&self, query: &HintQuery, entries: &[StoryEntry]) -> Vec<ContextHint> {
        let mut hints = Vec::new();
        for entry in entries {
            let Some(hit) = match_story_entry(&query.nearby_text, &query.lookback_text, entry)
            else {
                continue;
            };
            if hit.score < self.minimum_dwell_score {
                continue;
            }
            let (kind, source) = match entry.kind {
                StoryEntryKind::Character => (HintKind::CharacterState, "人物"),
                StoryEntryKind::Setting => (HintKind::WorldRule, "设定"),
                StoryEntryKind::Foreshadow => (HintKind::OpenForeshadowing, "伏笔"),
            };
            let summary = if entry.summary.is_empty() {
                format!("{} · 预先设定", entry.title)
            } else {
                entry.summary.clone()
            };
            hints.push(hint(
                kind,
                &entry.title,
                &summary,
                source,
                &hit.reason,
                hit.score,
                query,
            ));
        }
        hints.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then(kind_rank(left.kind).cmp(&kind_rank(right.kind)))
                .then(left.title.cmp(&right.title))
        });
        hints.truncate(query.limit.clamp(1, 6));
        hints
    }
}

fn hint(
    kind: HintKind,
    title: &str,
    summary: &str,
    source_label: &str,
    match_reason: &str,
    score: f32,
    query: &HintQuery,
) -> ContextHint {
    ContextHint {
        id: format!("hint-{}-{}", query.generation, title),
        kind,
        title: title.into(),
        summary: summary.into(),
        source_label: source_label.into(),
        match_reason: match_reason.into(),
        confidence: score.clamp(0.0, 1.0),
        score: score.max(0.01),
        generation: query.generation,
        revision: Revision(query.work_ref.revision.0),
        actions: vec![
            HintAction::ExpandSource,
            HintAction::Pin,
            HintAction::Snooze,
            HintAction::Ignore,
            HintAction::MarkWrong,
        ],
    }
}

fn name_score(query: &HintQuery, entity: &CanonEntity) -> f32 {
    if query.nearby_text.contains(&entity.canonical_name) {
        return 1.0;
    }
    for alias in &entity.aliases {
        if query.nearby_text.contains(alias) {
            return 0.92;
        }
    }
    0.0
}

fn fact_relevant(query: &HintQuery, predicate: &str, value: &str) -> bool {
    query.nearby_text.contains(predicate) || query.nearby_text.contains(value)
}

fn kind_rank(kind: HintKind) -> u8 {
    match kind {
        HintKind::CharacterState => 0,
        HintKind::WorldRule => 1,
        HintKind::OpenForeshadowing => 2,
        _ => 9,
    }
}
