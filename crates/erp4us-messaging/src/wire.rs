use crate::{Error, Route, Scope, Subject, Version, VersionReq, subject::token};
use async_nats::HeaderMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Core,
    JetStream(&'static str),
}

pub trait Contract: Serialize + DeserializeOwned {
    const ROUTE: Route<'static>;
    const SCHEMA_VERSION: &'static str;
    const PROTOCOL_SUPPORT: &'static str;
    const SCHEMA_SUPPORT: &'static str;
    const DELIVERY: Delivery;

    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope<T> {
    pub message_id: Uuid,
    pub message_type: String,
    pub protocol_version: Version,
    pub schema_version: Version,
    pub service_version: Version,
    pub timestamp: String,
    pub source_service: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<Actor>,
    pub correlation_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracestate: Option<String>,
    pub payload: T,
}

impl<T: Contract> Envelope<T> {
    /// Start a new system-scoped correlation chain. Persist this envelope before retrying durable sends.
    pub fn system(
        payload: T,
        protocol: Version,
        source: &str,
        service: Version,
    ) -> Result<Self, Error> {
        let schema = Version::parse(T::SCHEMA_VERSION).map_err(|_| Error::InvalidContract)?;
        let message_id = Uuid::new_v4();
        let envelope = Self {
            message_id,
            message_type: T::ROUTE.subject(protocol.major, Scope::System, schema.major)?,
            protocol_version: protocol,
            schema_version: schema,
            service_version: service,
            timestamp: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .map_err(|_| Error::InvalidEnvelope("timestamp"))?,
            source_service: source.into(),
            tenant_id: None,
            organization_id: None,
            actor: None,
            correlation_id: message_id,
            causation_id: None,
            traceparent: None,
            tracestate: None,
            payload,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn subject(&self) -> Result<String, Error> {
        T::ROUTE.subject(
            self.protocol_version.major,
            self.tenant_id.map_or(Scope::System, Scope::Tenant),
            self.schema_version.major,
        )
    }

    pub fn validate(&self) -> Result<(), Error> {
        let check = |version: &Version, requirement: &str, unsupported| {
            let requirement = VersionReq::parse(requirement).map_err(|_| Error::InvalidContract)?;
            if version.pre.is_empty() && version.build.is_empty() && requirement.matches(version) {
                Ok(())
            } else {
                Err(unsupported)
            }
        };
        check(
            &self.protocol_version,
            T::PROTOCOL_SUPPORT,
            Error::UnsupportedProtocol,
        )?;
        check(
            &self.schema_version,
            T::SCHEMA_SUPPORT,
            Error::UnsupportedSchema,
        )?;
        if self.subject()? != self.message_type {
            return Err(Error::InvalidEnvelope("messageType"));
        }
        if self.message_id.is_nil()
            || self.correlation_id.is_nil()
            || self.causation_id.is_some_and(|id| id.is_nil())
            || self.organization_id.is_some_and(|id| id.is_nil())
            || (self.tenant_id.is_none() && self.organization_id.is_some())
        {
            return Err(Error::InvalidEnvelope("identity"));
        }
        if self.tenant_id.is_some() && self.actor.is_none()
            || self.actor.as_ref().is_some_and(|actor| {
                actor.id.trim().is_empty()
                    || !matches!(actor.kind.as_str(), "user" | "service" | "agent")
            })
        {
            return Err(Error::InvalidEnvelope("actor"));
        }
        if self.source_service.len() > 127 || !self.source_service.split('.').all(token) {
            return Err(Error::InvalidEnvelope("sourceService"));
        }
        if OffsetDateTime::parse(&self.timestamp, &Rfc3339).is_err() {
            return Err(Error::InvalidEnvelope("timestamp"));
        }
        if self
            .traceparent
            .as_deref()
            .is_some_and(|value| !valid_traceparent(value))
            || self
                .tracestate
                .as_deref()
                .is_some_and(|value| self.traceparent.is_none() || !valid_tracestate(value))
        {
            return Err(Error::InvalidEnvelope("trace context"));
        }
        T::validate(&self.payload)
    }

    pub(crate) fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in self.metadata() {
            if let Some(value) = value {
                headers.insert(key, value);
            }
        }
        headers
    }

    fn metadata(&self) -> [(&'static str, Option<String>); 12] {
        [
            (
                "Erp4us-Protocol-Version",
                Some(self.protocol_version.to_string()),
            ),
            (
                "Erp4us-Schema-Version",
                Some(self.schema_version.to_string()),
            ),
            (
                "Erp4us-Service-Version",
                Some(self.service_version.to_string()),
            ),
            ("Erp4us-Message-Id", Some(self.message_id.to_string())),
            (
                "Erp4us-Correlation-Id",
                Some(self.correlation_id.to_string()),
            ),
            (
                "Erp4us-Causation-Id",
                self.causation_id.map(|id| id.to_string()),
            ),
            ("Erp4us-Source", Some(self.source_service.clone())),
            ("Erp4us-Tenant-Id", self.tenant_id.map(|id| id.to_string())),
            (
                "Erp4us-Organization-Id",
                self.organization_id.map(|id| id.to_string()),
            ),
            ("Content-Type", Some("application/json".into())),
            ("traceparent", self.traceparent.clone()),
            ("tracestate", self.tracestate.clone()),
        ]
    }

    pub fn decode(subject: &str, headers: &HeaderMap, body: &[u8]) -> Result<Self, Error> {
        let subject_parts: Subject = subject.parse()?;
        let envelope: Self = serde_json::from_slice(body).map_err(Error::Deserialize)?;
        envelope.validate()?;
        if envelope.message_type != subject
            || envelope.protocol_version.major != subject_parts.protocol_major
            || envelope.schema_version.major != subject_parts.schema_major
        {
            return Err(Error::InvalidEnvelope("subject mapping"));
        }
        for (key, value) in envelope.metadata() {
            if single(headers, key)? != value.as_deref() {
                return Err(Error::InvalidHeader(key));
            }
        }
        if matches!(T::DELIVERY, Delivery::JetStream(_)) {
            let id = envelope.message_id.to_string();
            let key = single(headers, "Nats-Msg-Id")?;
            let replay = key
                .and_then(|key| key.strip_prefix("replay-"))
                .and_then(|key| key.split_once('-'))
                .is_some_and(|(sequence, suffix)| {
                    sequence
                        .parse::<u64>()
                        .is_ok_and(|n| n > 0 && n.to_string() == sequence)
                        && suffix == id
                });
            if key != Some(&id) && !replay {
                return Err(Error::InvalidHeader("Nats-Msg-Id"));
            }
        }
        Ok(envelope)
    }
}

fn single<'a>(headers: &'a HeaderMap, key: &'static str) -> Result<Option<&'a str>, Error> {
    let mut values = headers
        .iter()
        .filter(|(name, _)| name.to_string().eq_ignore_ascii_case(key))
        .flat_map(|(_, values)| values);
    let value = values.next().map(|value| value.as_str());
    if values.next().is_some() {
        return Err(Error::InvalidHeader(key));
    }
    Ok(value)
}

pub fn valid_traceparent(value: &str) -> bool {
    let mut fields = value.split('-');
    let hex = |field: Option<&str>, len, nonzero: bool| {
        field.is_some_and(|field| {
            field.len() == len
                && field
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                && (!nonzero || field.bytes().any(|b| b != b'0'))
        })
    };
    fields.next() == Some("00")
        && hex(fields.next(), 32, true)
        && hex(fields.next(), 16, true)
        && hex(fields.next(), 2, false)
        && fields.next().is_none()
}

fn valid_tracestate(value: &str) -> bool {
    if value.len() > 512 {
        return false;
    }
    let mut seen = std::collections::HashSet::new();
    let key_tail = |b: u8| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'*' | b'/')
    };
    value.split(',').all(|entry| {
        let Some((key, value)) = entry.trim_matches([' ', '\t']).split_once('=') else {
            return false;
        };
        let key = key.trim_end_matches([' ', '\t']);
        let valid_key = |part: &str, max: usize, digit: bool| {
            part.len() <= max
                && part
                    .bytes()
                    .next()
                    .is_some_and(|b| b.is_ascii_lowercase() || digit && b.is_ascii_digit())
                && part.bytes().all(key_tail)
        };
        let value = value.trim_start_matches([' ', '\t']);
        let valid = if let Some((tenant, system)) = key.split_once('@') {
            valid_key(tenant, 241, true) && valid_key(system, 14, false)
        } else {
            valid_key(key, 256, false)
        };
        valid
            && seen.len() < 32
            && seen.insert(key)
            && !value.is_empty()
            && value.len() <= 256
            && value
                .bytes()
                .all(|b| (0x20..=0x7e).contains(&b) && !matches!(b, b',' | b'='))
    })
}
