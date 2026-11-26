use crate::Error;
use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! tokens {
    ($name:ident, $($variant:ident => $wire:literal),+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const fn token(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }
        }
        impl FromStr for $name {
            type Err = Error;
            fn from_str(value: &str) -> Result<Self, Error> {
                match value { $($wire => Ok(Self::$variant),)+ _ => Err(Error::InvalidSubject) }
            }
        }
    }
}

tokens!(Layer, Config => "config", Business => "business", Diagnostics => "diagnostics");
tokens!(Kind, Event => "event", Command => "command", Query => "query");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    System,
    Tenant(Uuid),
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => f.write_str("system"),
            Self::Tenant(id) => id.fmt(f),
        }
    }
}

pub(crate) fn token(value: &str) -> bool {
    value.len() <= 63
        && value.starts_with(|c: char| c.is_ascii_lowercase())
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn major(value: &str) -> Result<u64, Error> {
    let value = value.strip_prefix('v').ok_or(Error::InvalidSubject)?;
    if value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::InvalidSubject);
    }
    value.parse().map_err(|_| Error::InvalidSubject)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Route<'a> {
    pub layer: Layer,
    pub domain: &'a str,
    pub kind: Kind,
    pub resource: &'a str,
    pub action: &'a str,
}

impl Route<'_> {
    pub fn subject(self, protocol: u64, scope: Scope, schema: u64) -> Result<String, Error> {
        self.build(protocol, &scope.to_string(), schema)
            .and_then(|subject| {
                subject.parse::<Subject>()?;
                Ok(subject)
            })
    }

    /// Subscribe across scopes, but never across protocol or schema majors.
    pub fn all_scopes(self, protocol: u64, schema: u64) -> Result<String, Error> {
        self.build(protocol, "*", schema)
    }

    fn build(self, protocol: u64, scope: &str, schema: u64) -> Result<String, Error> {
        if protocol == 0
            || schema == 0
            || ![self.domain, self.resource, self.action]
                .into_iter()
                .all(token)
        {
            return Err(Error::InvalidSubject);
        }
        Ok(format!(
            "v{protocol}.{}.{scope}.{}.{}.{}.{}.v{schema}",
            self.layer.token(),
            self.domain,
            self.kind.token(),
            self.resource,
            self.action
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subject {
    pub protocol_major: u64,
    pub layer: Layer,
    pub scope: Scope,
    pub domain: String,
    pub kind: Kind,
    pub resource: String,
    pub action: String,
    pub schema_major: u64,
}

impl FromStr for Subject {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut segments = value.split('.');
        let [
            Some(protocol),
            Some(layer),
            Some(scope),
            Some(domain),
            Some(kind),
            Some(resource),
            Some(action),
            Some(schema),
            None,
        ] = std::array::from_fn(|_| segments.next())
        else {
            return Err(Error::InvalidSubject);
        };
        if ![domain, resource, action].into_iter().all(token) {
            return Err(Error::InvalidSubject);
        }
        let scope = if scope == "system" {
            Scope::System
        } else {
            let id = Uuid::parse_str(scope).map_err(|_| Error::InvalidSubject)?;
            if id.is_nil() || id.to_string() != scope {
                return Err(Error::InvalidSubject);
            }
            Scope::Tenant(id)
        };
        Ok(Self {
            protocol_major: major(protocol)?,
            layer: layer.parse()?,
            scope,
            domain: domain.into(),
            kind: kind.parse()?,
            resource: resource.into(),
            action: action.into(),
            schema_major: major(schema)?,
        })
    }
}
