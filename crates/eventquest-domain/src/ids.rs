use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generates a `Copy`, `Uuid`-backed newtype so callers can't accidentally
/// pass an `EventId` where a `CheckpointId` is expected — a real risk in a
/// schema this densely cross-referenced (spec §13).
macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

uuid_id!(OrganizationId);
uuid_id!(EventId);
uuid_id!(CheckpointId);
uuid_id!(ParticipantId);
uuid_id!(GrantId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_through_uuid() {
        let raw = Uuid::new_v4();
        let event_id = EventId::from(raw);
        assert_eq!(Uuid::from(event_id), raw);
    }

    #[test]
    fn new_ids_are_unique() {
        assert_ne!(EventId::new(), EventId::new());
    }
}
