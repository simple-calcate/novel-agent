use super::Repository;
use crate::StorageError;
use novel_domain::{
    CorrectionRecord, PreferenceRule, PreferenceRuleId, PreferenceStatus, ProjectId,
};
use rusqlite::params;
use serde_json::json;

impl Repository {
    pub fn save_preference_rule(
        &self,
        project_id: &ProjectId,
        rule: &PreferenceRule,
    ) -> Result<PreferenceRule, StorageError> {
        if let Some(mut existing) = self.find_preference_by_rule(project_id, &rule.rule)? {
            for evidence in &rule.evidence_proposals {
                if !existing.evidence_proposals.contains(evidence) {
                    existing.evidence_proposals.push(evidence.clone());
                }
            }
            if existing.status == PreferenceStatus::Candidate
                && existing.evidence_proposals.len() >= 2
            {
                existing.status = PreferenceStatus::Confirmed;
            }
            existing.updated_at = chrono::Utc::now();
            self.connection.execute(
                "UPDATE preference_rules
                 SET status = ?2, data_json = ?3
                 WHERE id = ?1",
                params![
                    existing.id.to_string(),
                    status_name(existing.status),
                    data_json(&existing)?,
                ],
            )?;
            return Ok(existing);
        }

        self.connection.execute(
            "INSERT INTO preference_rules(id, project_id, scope, rule, status, data_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                rule.id.to_string(),
                project_id.to_string(),
                serde_json::to_string(&rule.scope)?,
                rule.rule,
                status_name(rule.status),
                data_json(rule)?,
            ],
        )?;
        Ok(rule.clone())
    }

    pub fn list_preference_rules(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<PreferenceRule>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, scope, rule, status, data_json
             FROM preference_rules
             WHERE project_id = ?1
             ORDER BY id DESC",
        )?;
        let rows = statement.query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        let mut rules = Vec::new();
        for row in rows {
            let (id, scope, rule, status, data) = row?;
            let Ok(id) = id.parse::<PreferenceRuleId>() else {
                continue;
            };
            let extra: serde_json::Value = serde_json::from_str(&data).unwrap_or(json!({}));
            let evidence = extra.get("evidenceProposals").cloned().unwrap_or(json!([]));
            let created_at = extra
                .get("createdAt")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let updated_at = extra
                .get("updatedAt")
                .and_then(|value| value.as_str())
                .unwrap_or(created_at);
            rules.push(PreferenceRule {
                id,
                scope: serde_json::from_str(&scope).unwrap_or(
                    novel_domain::PreferenceScope::Project {
                        project_id: project_id.to_string(),
                    },
                ),
                rule,
                status: parse_status(&status),
                evidence_proposals: serde_json::from_value(evidence).unwrap_or_default(),
                created_at: super::parse_rfc3339(created_at),
                updated_at: super::parse_rfc3339(updated_at),
            });
        }
        Ok(rules)
    }

    pub fn set_preference_status(
        &self,
        project_id: &ProjectId,
        rule_id: &PreferenceRuleId,
        status: PreferenceStatus,
    ) -> Result<PreferenceRule, StorageError> {
        let mut rules = self.list_preference_rules(project_id)?;
        let Some(rule) = rules.iter_mut().find(|item| &item.id == rule_id) else {
            return Err(novel_domain::DomainError::NotFound(format!("preference {rule_id}")).into());
        };
        rule.status = status;
        rule.updated_at = chrono::Utc::now();
        self.connection.execute(
            "UPDATE preference_rules SET status = ?2, data_json = ?3 WHERE id = ?1",
            params![
                rule.id.to_string(),
                status_name(rule.status),
                data_json(rule)?,
            ],
        )?;
        Ok(rule.clone())
    }

    pub fn save_correction(
        &self,
        project_id: &ProjectId,
        record: &CorrectionRecord,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO correction_records(
                project_id, proposal_id, ai_text, human_text, diff_summary, context_excerpt, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project_id.to_string(),
                record.proposal_id.to_string(),
                record.ai_text,
                record.human_text,
                record.diff_summary,
                record.context_excerpt,
                record.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn find_preference_by_rule(
        &self,
        project_id: &ProjectId,
        rule: &str,
    ) -> Result<Option<PreferenceRule>, StorageError> {
        Ok(self
            .list_preference_rules(project_id)?
            .into_iter()
            .find(|item| item.rule == rule && item.status != PreferenceStatus::Disabled))
    }
}

fn data_json(rule: &PreferenceRule) -> Result<String, StorageError> {
    Ok(json!({
        "evidenceProposals": rule.evidence_proposals,
        "createdAt": rule.created_at.to_rfc3339(),
        "updatedAt": rule.updated_at.to_rfc3339(),
    })
    .to_string())
}

fn status_name(status: PreferenceStatus) -> &'static str {
    match status {
        PreferenceStatus::Candidate => "candidate",
        PreferenceStatus::Confirmed => "confirmed",
        PreferenceStatus::Disabled => "disabled",
    }
}

fn parse_status(value: &str) -> PreferenceStatus {
    match value {
        "confirmed" => PreferenceStatus::Confirmed,
        "disabled" => PreferenceStatus::Disabled,
        _ => PreferenceStatus::Candidate,
    }
}
