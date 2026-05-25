use crate::{models::Role, util};

/// Lilith Passport login credentials.
#[derive(Debug, Clone)]
pub struct Credentials {
    email: String,
    password: String,
}

impl Credentials {
    /// Use the password value expected by Lilith Passport.
    pub fn new(email: impl Into<String>, password: impl Into<String>) -> Self {
        Self { email: email.into(), password: password.into().trim().to_ascii_lowercase() }
    }

    pub(crate) fn email(&self) -> &str {
        &self.email
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }

    pub(crate) fn visitor_id(&self) -> String {
        util::md5_hex(self.email.as_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthSession {
    pauthorization: String,
}

impl AuthSession {
    pub(crate) fn from_pauthorization(token: impl Into<String>) -> Self {
        Self { pauthorization: util::normalize_bearer_token(token) }
    }

    pub(crate) fn bearer(&self) -> String {
        format!("Bearer {}", self.pauthorization)
    }

    fn into_token(self) -> String {
        self.pauthorization
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundSession {
    pauthorization: String,
    bauthorization: String,
    role: Role,
}

impl BoundSession {
    pub(crate) fn from_tokens(
        auth_session: AuthSession,
        bauthorization: impl Into<String>,
        role: Role,
    ) -> Self {
        Self {
            pauthorization: auth_session.into_token(),
            bauthorization: util::normalize_bearer_token(bauthorization),
            role,
        }
    }

    pub(crate) fn pauth_bearer(&self) -> String {
        format!("Bearer {}", self.pauthorization)
    }

    pub(crate) fn bauth_bearer(&self) -> String {
        format!("Bearer {}", self.bauthorization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_normalize_password() {
        let credentials =
            Credentials::new("person@example.com", " 5F4DCC3B5AA765D61D8327DEB882CF99 ");

        assert_eq!(credentials.password(), "5f4dcc3b5aa765d61d8327deb882cf99");
    }
}
