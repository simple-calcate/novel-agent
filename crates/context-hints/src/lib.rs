//! 写作时的上下文浮带：按附近文本匹配人物状态、世界规则与未兑现伏笔。

use novel_domain::{
    CanonEntity, CanonFact, ContextHint, HintAction, HintKind, PlotThread, PlotThreadStatus,
    Revision, WorkContextRef,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HintQuery {
    pub work_ref: WorkContextRef,
    pub nearby_text: String,
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
            hints.push(hint(
                HintKind::CharacterState,
                &matched_name,
                &format!("{} 可能出现在当前写作环境中", entity.canonical_name),
                "人物设定",
                "检测到人物名称或别名",
                score,
                query,
            ));
        }

        for fact in facts {
            let text = fact.value.to_string();
            if fact_relevant(query, &fact.predicate, &text) {
                hints.push(hint(
                    HintKind::WorldRule,
                    &fact.predicate,
                    &format!("{}: {}", fact.predicate, text),
                    "正史设定",
                    "当前文字与设定字段匹配",
                    fact.confidence,
                    query,
                ));
            }
        }

        for thread in threads {
            if thread.status == PlotThreadStatus::Open {
                hints.push(hint(
                    HintKind::OpenForeshadowing,
                    &thread.title,
                    &format!("伏笔「{}」仍未兑现", thread.title),
                    "伏笔看板",
                    "存在开放伏笔",
                    0.75,
                    query,
                ));
            }
        }

        hints.sort_by(|left, right| right.score.total_cmp(&left.score));
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
