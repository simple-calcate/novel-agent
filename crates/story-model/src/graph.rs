use novel_domain::{CanonEntity, EntityId, Relationship};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
pub struct StoryGraph {
    entities: BTreeMap<EntityId, CanonEntity>,
    outgoing: BTreeMap<EntityId, Vec<Relationship>>,
    incoming: BTreeMap<EntityId, Vec<Relationship>>,
}

impl StoryGraph {
    pub fn insert_entity(&mut self, entity: CanonEntity) {
        self.entities.insert(entity.id.clone(), entity);
    }

    pub fn insert_relationship(&mut self, relationship: Relationship) {
        self.outgoing
            .entry(relationship.from.clone())
            .or_default()
            .push(relationship.clone());
        self.incoming
            .entry(relationship.to.clone())
            .or_default()
            .push(relationship);
    }

    pub fn find_by_name_or_alias(&self, name: &str) -> Vec<&CanonEntity> {
        self.entities
            .values()
            .filter(|entity| {
                entity.canonical_name == name || entity.aliases.iter().any(|alias| alias == name)
            })
            .collect()
    }

    pub fn related_entities(&self, entity_id: &EntityId, depth: usize) -> BTreeSet<EntityId> {
        let mut visited = BTreeSet::new();
        let mut frontier = vec![entity_id.clone()];

        for _ in 0..depth {
            let mut next = Vec::new();
            for id in frontier {
                if !visited.insert(id.clone()) {
                    continue;
                }
                if let Some(edges) = self.outgoing.get(&id) {
                    next.extend(edges.iter().map(|edge| edge.to.clone()));
                }
                if let Some(edges) = self.incoming.get(&id) {
                    next.extend(edges.iter().map(|edge| edge.from.clone()));
                }
            }
            frontier = next;
        }

        visited.remove(entity_id);
        visited
    }
}
