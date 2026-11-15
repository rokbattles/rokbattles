//! Role names, membership sets, and requirements for route extractors.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

/// A role associated with an authenticated user.
///
/// Role names are serialized as snake_case strings. Their declaration order
/// determines serialization order, not a permission hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The implicit role of every authenticated user.
    User,
    /// Premium membership.
    Premium,
    /// Content creator membership.
    ContentCreator,
    /// Developer program membership.
    DeveloperProgram,
    /// Administrator membership, with no implicit access to other roles.
    Admin,
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Serde's derived enum parser also accepts objects such as {"admin": null}.
        // Requiring a string keeps stored roles consistent with the API format.
        let value = String::deserialize(deserializer)?;

        match value.as_str() {
            "user" => Ok(Self::User),
            "premium" => Ok(Self::Premium),
            "content_creator" => Ok(Self::ContentCreator),
            "developer_program" => Ok(Self::DeveloperProgram),
            "admin" => Ok(Self::Admin),
            _ => Err(serde::de::Error::custom("unknown role")),
        }
    }
}

/// A set of effective roles for an authenticated user.
///
/// The set always contains [`Role::User`] and removes duplicate roles. All other
/// roles are independent, including [`Role::Admin`].
///
/// Deserialization accepts an array of role names or `null`. An empty array or
/// `null` produces only `User`; unknown names and other value types return an error.
/// Stored fields use `#[serde(default)]` to give missing roles the same behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Roles(BTreeSet<Role>);

impl Default for Roles {
    fn default() -> Self {
        Self(BTreeSet::from([Role::User]))
    }
}

impl FromIterator<Role> for Roles {
    fn from_iter<T: IntoIterator<Item = Role>>(iter: T) -> Self {
        let mut roles = Self::default();
        roles.0.extend(iter);

        roles
    }
}

impl<'de> Deserialize<'de> for Roles {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let roles = Option::<BTreeSet<Role>>::deserialize(deserializer)?;

        Ok(roles.into_iter().flatten().collect())
    }
}

impl Roles {
    /// Returns `true` if the user has the given role.
    pub fn contains(&self, role: Role) -> bool {
        self.0.contains(&role)
    }
}

/// Selects the role checked by [`super::RequireRole`].
pub trait RequiredRole: Send + Sync {
    /// The role required by this marker type.
    const ROLE: Role;
}

/// A [`Role::User`] requirement for [`super::RequireRole`].
pub struct User;

impl RequiredRole for User {
    const ROLE: Role = Role::User;
}

/// A [`Role::Premium`] requirement for [`super::RequireRole`].
pub struct Premium;

impl RequiredRole for Premium {
    const ROLE: Role = Role::Premium;
}

/// A [`Role::ContentCreator`] requirement for [`super::RequireRole`].
pub struct ContentCreator;

impl RequiredRole for ContentCreator {
    const ROLE: Role = Role::ContentCreator;
}

/// A [`Role::DeveloperProgram`] requirement for [`super::RequireRole`].
pub struct DeveloperProgram;

impl RequiredRole for DeveloperProgram {
    const ROLE: Role = Role::DeveloperProgram;
}

/// A [`Role::Admin`] requirement for [`super::RequireRole`].
pub struct Admin;

impl RequiredRole for Admin {
    const ROLE: Role = Role::Admin;
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{Bson, from_bson};

    use super::*;

    #[test]
    fn every_exact_role_name_deserializes_from_bson() {
        for (name, role) in [
            ("user", Role::User),
            ("premium", Role::Premium),
            ("content_creator", Role::ContentCreator),
            ("developer_program", Role::DeveloperProgram),
            ("admin", Role::Admin),
        ] {
            let parsed: Role = from_bson(Bson::String(name.to_string())).expect("known role");

            assert_eq!(parsed, role);
        }
    }
}
