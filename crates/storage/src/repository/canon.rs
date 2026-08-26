use super::{parse_project_id, Repository};
use crate::StorageError;
use novel_domain::{
    CanonEntity, CanonFact, CanonProposal, ChapterId, DomainError, EntityId, EntityKind,
    ExtractedMention, FactId, FactStatus, PlotThread, ProjectId, Revision, SourceRef,
};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

impl Repository {
    pub fn list_canon_entities(&self) -> Result<Vec<CanonEntity>, StorageError> {
        self.query_entities(None)
    }

    pub fn list_canon_entities_for_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<CanonEntity>, StorageError> {
        self.query_entities(Some(project_id))
    }

    fn query_entities(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<Vec<CanonEntity>, StorageError> {
        let sql = if project_id.is_some() {
            "SELECT id, project_id, branch_id, kind, canonical_name, aliases_json, attributes_json
             FROM canon_entities WHERE project_id = ?1"
        } else {
            "SELECT id, project_id, branch_id, kind, canonical_name, aliases_json, attributes_json
             FROM canon_entities"
        };
        let mut statement = self.connection.prepare(sql)?;
        let mut rows = match project_id {
            Some(id) => statement.query(params![id.to_string()])?,
            None => statement.query([])?,
        };
        let mut entities = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let project: String = row.get(1)?;
            let branch_id: String = row.get(2)?;
            let kind: String = row.get(3)?;
            let canonical_name: String = row.get(4)?;
            let aliases: String = row.get(5)?;
            let attributes: String = row.get(6)?;
            let Ok(kind) = serde_json::from_value::<EntityKind>(serde_json::Value::String(kind))
            else {
                continue;
            };
            let Some(id) = id.parse().ok() else {
                continue;
            };
            let project_id = parse_project_id_lenient(&project)?;
            entities.push(CanonEntity {
                id,
                project_id,
                branch_id,
                kind,
                canonical_name,
                aliases: serde_json::from_str(&aliases).unwrap_or_default(),
                attributes: serde_json::from_str(&attributes).unwrap_or_default(),
            });
        }
        Ok(entities)
    }

    pub fn list_canon_facts(&self) -> Result<Vec<CanonFact>, StorageError> {
        self.query_facts(None, None)
    }

    pub fn list_canon_facts_for_project(
        &self,
        project_id: &ProjectId,
        status: Option<FactStatus>,
    ) -> Result<Vec<CanonFact>, StorageError> {
        self.query_facts(Some(project_id), status)
    }

    fn query_facts(
        &self,
        project_id: Option<&ProjectId>,
        status: Option<FactStatus>,
    ) -> Result<Vec<CanonFact>, StorageError> {
        let sql = match (project_id, status) {
            (Some(_), Some(_)) => {
                "SELECT fact_json FROM canon_facts WHERE project_id = ?1 AND status = ?2"
            }
            (Some(_), None) => "SELECT fact_json FROM canon_facts WHERE project_id = ?1",
            (None, Some(_)) => "SELECT fact_json FROM canon_facts WHERE status = ?1",
            (None, None) => "SELECT fact_json FROM canon_facts",
        };
        let mut statement = self.connection.prepare(sql)?;
        let status_name = status.map(fact_status_name);
        let mut jsons = Vec::new();
        match (project_id, status_name) {
            (Some(pid), Some(st)) => {
                let mut rows = statement.query(params![pid.to_string(), st])?;
                while let Some(row) = rows.next()? {
                    jsons.push(row.get::<_, String>(0)?);
                }
            }
            (Some(pid), None) => {
                let mut rows = statement.query(params![pid.to_string()])?;
                while let Some(row) = rows.next()? {
                    jsons.push(row.get::<_, String>(0)?);
                }
            }
            (None, Some(st)) => {
                let mut rows = statement.query(params![st])?;
                while let Some(row) = rows.next()? {
                    jsons.push(row.get::<_, String>(0)?);
                }
            }
            (None, None) => {
                let mut rows = statement.query([])?;
                while let Some(row) = rows.next()? {
                    jsons.push(row.get::<_, String>(0)?);
                }
            }
        }
        let mut facts = Vec::new();
        for json in jsons {
            if let Ok(fact) = serde_json::from_str::<CanonFact>(&json) {
                facts.push(fact);
            }
        }
        Ok(facts)
    }

    pub fn list_plot_threads(&self) -> Result<Vec<PlotThread>, StorageError> {
        self.query_plot_threads(None)
    }

    pub fn list_plot_threads_for_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<PlotThread>, StorageError> {
        self.query_plot_threads(Some(project_id))
    }

    fn query_plot_threads(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<Vec<PlotThread>, StorageError> {
        let sql = if project_id.is_some() {
            "SELECT data_json FROM plot_threads WHERE project_id = ?1"
        } else {
            "SELECT data_json FROM plot_threads"
        };
        let mut statement = self.connection.prepare(sql)?;
        let mut jsons = Vec::new();
        match project_id {
            Some(id) => {
                let mut rows = statement.query(params![id.to_string()])?;
                while let Some(row) = rows.next()? {
                    jsons.push(row.get::<_, String>(0)?);
                }
            }
            None => {
                let mut rows = statement.query([])?;
                while let Some(row) = rows.next()? {
                    jsons.push(row.get::<_, String>(0)?);
                }
            }
        }
        let mut threads = Vec::new();
        for json in jsons {
            if let Ok(thread) = serde_json::from_str::<PlotThread>(&json) {
                threads.push(thread);
            }
        }
        Ok(threads)
    }

    /// 用已接受的正史实体重建 FTS 检索索引，返回索引条数。
    pub fn rebuild_search_index(&self, project_id: &ProjectId) -> Result<u32, StorageError> {
        self.connection.execute(
            "DELETE FROM search_documents WHERE project_id = ?1",
            params![project_id.to_string()],
        )?;
        let inserted = self.connection.execute(
            "INSERT INTO search_documents(project_id, entity_kind, entity_id, title, body)
             SELECT ?1, kind, id, canonical_name, canonical_name || ' ' || aliases_json
             FROM canon_entities
             WHERE project_id = ?1
               AND id IN (
                   SELECT entity_id FROM canon_facts
                   WHERE project_id = ?1 AND status = 'accepted'
               )",
            params![project_id.to_string()],
        )?;
        Ok(inserted as u32)
    }

    pub fn propose_canon_mentions(
        &self,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        revision: Revision,
        mentions: &[ExtractedMention],
    ) -> Result<Vec<CanonProposal>, StorageError> {
        let mut created = Vec::new();
        for mention in mentions {
            let entity = self.find_or_create_entity(project_id, mention)?;
            if self.has_duplicate_fact(&entity.id, mention, chapter_id)? {
                continue;
            }
            let fact =
                self.insert_candidate_fact(&entity, project_id, chapter_id, revision, mention)?;
            created.push(to_proposal(&entity, &fact));
        }
        Ok(created)
    }

    pub fn list_canon_proposals(
        &self,
        project_id: &ProjectId,
        status: Option<FactStatus>,
    ) -> Result<Vec<CanonProposal>, StorageError> {
        let facts = self.list_canon_facts_for_project(project_id, status)?;
        let entities = self.list_canon_entities_for_project(project_id)?;
        let mut proposals = Vec::new();
        for fact in facts {
            let Some(entity) = entities.iter().find(|entity| entity.id == fact.entity_id) else {
                continue;
            };
            proposals.push(to_proposal(entity, &fact));
        }
        proposals.sort_by(|left, right| {
            left.entity_name
                .cmp(&right.entity_name)
                .then(left.predicate.cmp(&right.predicate))
        });
        Ok(proposals)
    }

    pub fn set_fact_status(
        &self,
        fact_id: &FactId,
        status: FactStatus,
    ) -> Result<CanonProposal, StorageError> {
        let json: String = self
            .connection
            .query_row(
                "SELECT fact_json FROM canon_facts WHERE id = ?1",
                [fact_id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| DomainError::NotFound(format!("canon fact {fact_id}")))?;
        let mut fact: CanonFact = serde_json::from_str(&json)?;
        fact.status = status;
        self.connection.execute(
            "UPDATE canon_facts SET status = ?2, fact_json = ?3 WHERE id = ?1",
            params![
                fact_id.to_string(),
                fact_status_name(status),
                serde_json::to_string(&fact)?
            ],
        )?;
        let entity = self.entity_by_id(&fact.entity_id)?;
        Ok(to_proposal(&entity, &fact))
    }

    fn find_or_create_entity(
        &self,
        project_id: &ProjectId,
        mention: &ExtractedMention,
    ) -> Result<CanonEntity, StorageError> {
        let kind = kind_name(&mention.entity_kind);
        let existing: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM canon_entities
                 WHERE project_id = ?1 AND canonical_name = ?2 AND kind = ?3",
                params![project_id.to_string(), mention.entity_name, kind],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            let entity_id: EntityId = id
                .parse()
                .map_err(|_| DomainError::Validation("bad entity id".into()))?;
            return self.entity_by_id(&entity_id);
        }
        let entity = CanonEntity {
            id: EntityId::new(),
            project_id: project_id.clone(),
            branch_id: "main".into(),
            kind: mention.entity_kind.clone(),
            canonical_name: mention.entity_name.clone(),
            aliases: Vec::new(),
            attributes: Default::default(),
        };
        self.connection.execute(
            "INSERT INTO canon_entities(
                id, project_id, branch_id, kind, canonical_name, aliases_json, attributes_json
            ) VALUES (?1, ?2, 'main', ?3, ?4, ?5, ?6)",
            params![
                entity.id.to_string(),
                project_id.to_string(),
                kind,
                entity.canonical_name,
                serde_json::to_string(&entity.aliases)?,
                serde_json::to_string(&entity.attributes)?,
            ],
        )?;
        Ok(entity)
    }

    fn insert_candidate_fact(
        &self,
        entity: &CanonEntity,
        project_id: &ProjectId,
        chapter_id: &ChapterId,
        revision: Revision,
        mention: &ExtractedMention,
    ) -> Result<CanonFact, StorageError> {
        let fact = CanonFact {
            id: FactId::new(),
            entity_id: entity.id.clone(),
            branch_id: "main".into(),
            predicate: mention.predicate.clone(),
            value: serde_json::Value::String(mention.object.clone()),
            status: FactStatus::Candidate,
            confidence: mention.confidence,
            source: SourceRef {
                chapter_id: chapter_id.clone(),
                block_id: None,
                revision,
                quote: mention.quote.clone(),
            },
            valid_from: None,
            valid_to: None,
            revision_from: revision,
            revision_to: None,
        };
        self.connection.execute(
            "INSERT INTO canon_facts(
                id, entity_id, branch_id, predicate, value_json, status, confidence,
                source_json, fact_json, project_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                fact.id.to_string(),
                fact.entity_id.to_string(),
                fact.branch_id,
                fact.predicate,
                serde_json::to_string(&fact.value)?,
                fact_status_name(fact.status),
                fact.confidence,
                serde_json::to_string(&fact.source)?,
                serde_json::to_string(&fact)?,
                project_id.to_string(),
            ],
        )?;
        Ok(fact)
    }

    fn has_duplicate_fact(
        &self,
        entity_id: &EntityId,
        mention: &ExtractedMention,
        chapter_id: &ChapterId,
    ) -> Result<bool, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT fact_json FROM canon_facts WHERE entity_id = ?1 AND predicate = ?2")?;
        let rows = statement
            .query_map(params![entity_id.to_string(), mention.predicate], |row| {
                row.get::<_, String>(0)
            })?;
        for row in rows {
            let Ok(fact) = serde_json::from_str::<CanonFact>(&row?) else {
                continue;
            };
            let object = match &fact.value {
                serde_json::Value::String(value) => value.as_str(),
                _ => continue,
            };
            if object == mention.object && &fact.source.chapter_id == chapter_id {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn entity_by_id(&self, entity_id: &EntityId) -> Result<CanonEntity, StorageError> {
        let row = self.connection.query_row(
            "SELECT id, project_id, branch_id, kind, canonical_name, aliases_json, attributes_json
             FROM canon_entities WHERE id = ?1",
            [entity_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )?;
        let (id, project, branch_id, kind, canonical_name, aliases, attributes) = row;
        let kind = serde_json::from_value::<EntityKind>(serde_json::Value::String(kind))
            .map_err(|_| DomainError::Validation("bad entity kind".into()))?;
        let id: EntityId = id
            .parse()
            .map_err(|_| DomainError::Validation("bad entity id".into()))?;
        Ok(CanonEntity {
            id,
            project_id: parse_project_id_lenient(&project)?,
            branch_id,
            kind,
            canonical_name,
            aliases: serde_json::from_str(&aliases).unwrap_or_default(),
            attributes: serde_json::from_str(&attributes).unwrap_or_default(),
        })
    }
}

fn parse_project_id_lenient(value: &str) -> Result<ProjectId, StorageError> {
    if value.is_empty() {
        return Ok(ProjectId(Uuid::nil()));
    }
    parse_project_id(value)
}

fn kind_name(kind: &EntityKind) -> String {
    match serde_json::to_value(kind) {
        Ok(serde_json::Value::String(name)) => name,
        _ => "character".into(),
    }
}

fn fact_status_name(status: FactStatus) -> &'static str {
    match status {
        FactStatus::Candidate => "candidate",
        FactStatus::Accepted => "accepted",
        FactStatus::Rejected => "rejected",
        FactStatus::Superseded => "superseded",
    }
}

fn to_proposal(entity: &CanonEntity, fact: &CanonFact) -> CanonProposal {
    let object = match &fact.value {
        serde_json::Value::String(value) => value.clone(),
        other => other.to_string(),
    };
    CanonProposal {
        fact_id: fact.id.clone(),
        entity_id: entity.id.clone(),
        project_id: entity.project_id.clone(),
        chapter_id: Some(fact.source.chapter_id.clone()),
        entity_name: entity.canonical_name.clone(),
        entity_kind: entity.kind.clone(),
        predicate: fact.predicate.clone(),
        object,
        quote: fact.source.quote.clone(),
        status: fact.status,
        confidence: fact.confidence,
    }
}
