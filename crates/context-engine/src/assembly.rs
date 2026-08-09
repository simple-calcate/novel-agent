use novel_domain::{
    ContextPackage, ContextSection, ContextSource, Revision, WorkContextRef,
};

pub struct AssemblyOptions {
    pub token_budget: u32,
}

pub fn assemble_context(
    work_ref: WorkContextRef,
    instruction: &str,
    current_scene: &str,
    pinned: &[String],
    retrieved: &[String],
    session_summaries: &[String],
    options: AssemblyOptions,
) -> ContextPackage {
    let mut sections = Vec::new();
    push_section(
        &mut sections,
        "当前指令",
        instruction,
        0,
        true,
        "用户当前输入",
    );
    push_section(
        &mut sections,
        "当前场景",
        current_scene,
        1,
        true,
        "当前章节/场景",
    );

    for (index, item) in pinned.iter().enumerate() {
        push_section(
            &mut sections,
            &format!("钉选设定 {}", index + 1),
            item,
            2,
            true,
            "作者钉选",
        );
    }
    for (index, item) in retrieved.iter().enumerate() {
        push_section(
            &mut sections,
            &format!("检索材料 {}", index + 1),
            item,
            10 + index as u32,
            false,
            "混合检索",
        );
    }
    for (index, item) in session_summaries.iter().enumerate() {
        push_section(
            &mut sections,
            &format!("会话摘要 {}", index + 1),
            item,
            100 + index as u32,
            false,
            "分层摘要",
        );
    }

    let mut used = 0;
    for section in &mut sections {
        let cost = section.source.token_cost;
        if !section.required && used + cost > options.token_budget {
            section.text.clear();
        } else {
            used += cost;
        }
    }

    ContextPackage {
        id: format!("ctx-{}", work_ref.revision.0),
        token_budget: options.token_budget,
        sources: sections.iter().map(|section| section.source.clone()).collect(),
        sections,
        work_ref,
    }
}

fn push_section(
    sections: &mut Vec<ContextSection>,
    title: &str,
    text: &str,
    priority: u32,
    required: bool,
    source_label: &str,
) {
    let token_cost = crate::estimate_tokens(text);
    sections.push(ContextSection {
        title: title.into(),
        text: text.into(),
        priority,
        required,
        source: ContextSource {
            label: source_label.into(),
            chapter_id: None,
            revision: Some(Revision(0)),
            confidence: 1.0,
            token_cost,
            reason: format!("按优先级 {priority} 装配"),
        },
    });
}
