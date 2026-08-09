use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(Self)
            }
        }
    };
}

typed_id!(ProjectId);
typed_id!(BookId);
typed_id!(VolumeId);
typed_id!(ChapterId);
typed_id!(SceneId);
typed_id!(BlockId);
typed_id!(AnnotationId);
typed_id!(EventId);
typed_id!(JobId);
typed_id!(WorkflowId);
typed_id!(PluginId);
typed_id!(AgentRunId);
typed_id!(SessionId);
typed_id!(ContextBlockId);
typed_id!(EntityId);
typed_id!(FactId);
typed_id!(StoryEventId);
typed_id!(PlotThreadId);
typed_id!(PreferenceRuleId);
typed_id!(ProposalId);
typed_id!(ResultObjectId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub u64);

impl Revision {
    pub const INITIAL: Self = Self(0);

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
