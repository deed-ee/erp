// Test-only public verification key and signed access token for a reserved
// .invalid issuer. The ephemeral signing key was not saved or deployed.
pub const ISSUER: &str = "https://receipt-source-test.invalid";
pub const SUBJECT: &str = "receipt-test-subject";
pub const AUDIENCE: &str = "procurement-test";
pub const JWKS: &str = concat!(
    r#"{"keys":[{"kty":"RSA","alg":"RS256","use":"sig","kid":"receipt-source-test","e":"AQAB","n":""#,
    "rEUxRzy_7FQ44hKbXwi2HmEUrQStSPlhMot66ieBt7qq_XDV45pdkfaaeQVPEYR_p9VL68QELNb49P6TdxCOtrC",
    "l873hgGCRYCGwiAjiWx-SCqSQO3OFKwZoXMccVgepfF9AKbXVR0xxIGZPPhEOrHH6dcKcjbr7PAs_BysPJ4uE-_",
    "yNKrYBibgrMUBzLzZJTFf-kJafaZCGIX2Jv7WYrpMyvrUHlpl0nDNvbEiMxAdDmdP9Zn9DBUIt03qOi8pxdJz",
    "vgWThshuwbD5aKw0aFOUKsdGct4Mm4wrBh1x4LSaIBlCKnl7K-6nLkuwQ0xz1R_YBQAw1m-qrV4DsuauheQ",
    r#""}]}"#,
);
pub const BEARER: &str = concat!(
    "Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6ImF0K2p3dCIsImtpZCI6InJlY2VpcHQtc291cmNlLXRlc3QifQ.",
    "eyJpc3MiOiJodHRwczovL3JlY2VpcHQtc291cmNlLXRlc3QuaW52YWxpZCIsImF1ZCI6InByb2N1cmVtZW50LX",
    "Rlc3QiLCJzdWIiOiJyZWNlaXB0LXRlc3Qtc3ViamVjdCIsImV4cCI6NDA3MDkwODgwMH0.",
    "Be1RJeSuHJMLe12aEoXflOeKrUCC9JK2XGkYpqmiKBerH_kg1Bv0d7rrsUmnJbaydrrpnqAbY8SYvJV3McvvbK",
    "j1N6YiqP1EDacEbUNq5iR69z6OFp3h6Hq42JtiblmQwrrI9tPxWxPel9qOIUjui9Xkc9YfQeYWq-GomtAc4CyZ",
    "LeMrGrsAk2WdogZ5EnvuxAZMJ_YOpl_Wn_mikuIHJ03biHPxTr38cuq6s2ZNlmq-uyGitpGkohP3HurbbMW85x",
    "WQyHnBOJExWiS4xzSl7e7S99UPfk7-Jj_OLoPtfixiBZiwtouACLafi2WzFVxNwkQ9Nn7G0l-7yr-emPXKsQ",
);
