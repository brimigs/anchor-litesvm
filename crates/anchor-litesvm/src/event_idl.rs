//! Auto-register a program's events for decoding straight from its Anchor IDL.
//!
//! [`AnchorContext::register_event`](crate::AnchorContext::register_event) is
//! the typed, one-call-per-event path. This module is the *zero-list* path: an
//! Anchor IDL already names every event, gives its 8-byte discriminator, and
//! describes its fields by type, so we can register a decoder for each without
//! the test ever naming them.
//!
//! The trade against the typed path: fields are formatted from the IDL's type
//! tags (`pubkey`, `u64`, ...) rather than the event's own `Debug`, so a field
//! whose type the decoder doesn't model (a `defined` struct, an `option`, a
//! `vec`) makes that event fall back to its raw base64 form rather than risk a
//! mis-aligned decode. Scalars, `bool`, `string`, and `pubkey` cover the common
//! event; `Pubkey`s still render base58 here and are aliased by the renderer.

use std::sync::Arc;

use litesvm_utils::EventRegistry;
use solana_program::pubkey::Pubkey;

/// The IDL field types the dynamic decoder models. Anything else parses to
/// [`IdlType::Unknown`], which makes the whole event un-decodable (a clean
/// fallback to raw, never a mis-aligned read).
#[derive(Clone, Copy)]
enum IdlType {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Bool,
    Str,
    Pubkey,
    Unknown,
}

fn parse_type(v: &serde_json::Value) -> IdlType {
    match v.as_str() {
        Some("u8") => IdlType::U8,
        Some("u16") => IdlType::U16,
        Some("u32") => IdlType::U32,
        Some("u64") => IdlType::U64,
        Some("u128") => IdlType::U128,
        Some("i8") => IdlType::I8,
        Some("i16") => IdlType::I16,
        Some("i32") => IdlType::I32,
        Some("i64") => IdlType::I64,
        Some("i128") => IdlType::I128,
        Some("bool") => IdlType::Bool,
        Some("string") => IdlType::Str,
        // Anchor has spelled it both ways across versions.
        Some("pubkey") | Some("publicKey") => IdlType::Pubkey,
        // A `defined` / `option` / `vec` / `array` object, or an unknown tag.
        _ => IdlType::Unknown,
    }
}

/// Take `n` bytes off the front of `cur`, advancing it, or `None` if short.
fn take<'a>(cur: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
    if cur.len() < n {
        return None;
    }
    let (head, tail) = cur.split_at(n);
    *cur = tail;
    Some(head)
}

/// Read one borsh-encoded value of `ty` off `cur`, formatted for display.
/// `None` on a short buffer or an [`IdlType::Unknown`] (which can't be skipped
/// without knowing its width).
fn read_value(cur: &mut &[u8], ty: IdlType) -> Option<String> {
    Some(match ty {
        IdlType::U8 => take(cur, 1)?[0].to_string(),
        IdlType::U16 => u16::from_le_bytes(take(cur, 2)?.try_into().ok()?).to_string(),
        IdlType::U32 => u32::from_le_bytes(take(cur, 4)?.try_into().ok()?).to_string(),
        IdlType::U64 => u64::from_le_bytes(take(cur, 8)?.try_into().ok()?).to_string(),
        IdlType::U128 => u128::from_le_bytes(take(cur, 16)?.try_into().ok()?).to_string(),
        IdlType::I8 => (take(cur, 1)?[0] as i8).to_string(),
        IdlType::I16 => i16::from_le_bytes(take(cur, 2)?.try_into().ok()?).to_string(),
        IdlType::I32 => i32::from_le_bytes(take(cur, 4)?.try_into().ok()?).to_string(),
        IdlType::I64 => i64::from_le_bytes(take(cur, 8)?.try_into().ok()?).to_string(),
        IdlType::I128 => i128::from_le_bytes(take(cur, 16)?.try_into().ok()?).to_string(),
        IdlType::Bool => (take(cur, 1)?[0] != 0).to_string(),
        IdlType::Str => {
            let len = u32::from_le_bytes(take(cur, 4)?.try_into().ok()?) as usize;
            let bytes = take(cur, len)?;
            format!("\"{}\"", String::from_utf8_lossy(bytes))
        }
        IdlType::Pubkey => {
            let bytes: [u8; 32] = take(cur, 32)?.try_into().ok()?;
            Pubkey::new_from_array(bytes).to_string()
        }
        IdlType::Unknown => return None,
    })
}

/// One event's fields as `(name, value)` pairs, decoded from the borsh body
/// (the bytes after the 8-byte discriminator) per `fields`. `None` if any field
/// is unmodelled or the buffer runs short, so the event keeps its raw form
/// rather than render garbage.
fn decode(body: &[u8], fields: &[(String, IdlType)]) -> Option<Vec<(String, String)>> {
    let mut cur = body;
    let mut pairs = Vec::with_capacity(fields.len());
    for (name, ty) in fields {
        pairs.push((name.clone(), read_value(&mut cur, *ty)?));
    }
    Some(pairs)
}

/// Parse a derived-`Debug` struct rendering into top-level `(field, value)`
/// pairs, best effort. `Name { a: 1, b: Foo { x: 2 } }` becomes
/// `[("a","1"), ("b","Foo { x: 2 }")]`. A non-struct rendering (a tuple, a
/// scalar, an enum variant) has no `{ .. }` to split, so it becomes a single
/// `("", whole)` pair and still renders as one value line.
///
/// Used by [`AnchorContext::register_event`](crate::AnchorContext::register_event),
/// whose decoder formats the event through `Debug`; the IDL path builds its
/// pairs directly and never needs this.
pub(crate) fn debug_to_pairs(s: &str) -> Vec<(String, String)> {
    let s = s.trim();
    let inner = match (s.find('{'), s.rfind('}')) {
        (Some(i), Some(j)) if j > i => s[i + 1..j].trim(),
        _ => return vec![(String::new(), s.to_string())],
    };
    split_top_level(inner, ',')
        .into_iter()
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|field| match field.find(':') {
            Some(i) => (
                field[..i].trim().to_string(),
                field[i + 1..].trim().to_string(),
            ),
            None => (String::new(), field.to_string()),
        })
        .collect()
}

/// Split `s` on `sep`, but only where it sits at brace depth zero and outside a
/// string literal, so a nested `Foo { x: 1, y: 2 }` or a `"a, b"` value stays
/// intact. Naive on escapes (a `\"` flips string state), which is fine for the
/// display-only use here.
fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let (mut depth, mut in_str, mut start) = (0i32, false, 0);
    let mut out = Vec::new();
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '{' | '[' | '(' if !in_str => depth += 1,
            '}' | ']' | ')' if !in_str => depth -= 1,
            c if c == sep && depth == 0 && !in_str => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Register a decoder for every event in `idl_json` into `registry`. Best
/// effort: events with a fully-modelled field list decode; the rest are still
/// registered by name but fall back to raw when their fields can't be read.
/// Returns the number of events registered. Panics only on invalid JSON (a test
/// setup error worth surfacing loudly).
pub(crate) fn register_all(registry: &mut EventRegistry, idl_json: &str) -> usize {
    let idl: serde_json::Value =
        serde_json::from_str(idl_json).expect("register_events_from_idl: invalid IDL JSON");

    // `name -> struct fields`, for resolving each event's body layout.
    let mut type_fields: std::collections::HashMap<&str, Vec<(String, IdlType)>> =
        std::collections::HashMap::new();
    for t in idl["types"].as_array().into_iter().flatten() {
        let Some(name) = t["name"].as_str() else {
            continue;
        };
        let fields = t["type"]["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| Some((f["name"].as_str()?.to_string(), parse_type(&f["type"]))))
            .collect();
        type_fields.insert(name, fields);
    }

    let mut count = 0;
    for ev in idl["events"].as_array().into_iter().flatten() {
        let Some(name) = ev["name"].as_str() else {
            continue;
        };
        let Some(disc) = ev["discriminator"]
            .as_array()
            .and_then(|a| {
                a.iter()
                    .map(|n| n.as_u64().map(|x| x as u8))
                    .collect::<Option<Vec<u8>>>()
            })
            .and_then(|v| <[u8; 8]>::try_from(v.as_slice()).ok())
        else {
            continue;
        };
        let fields: Vec<(String, IdlType)> = type_fields.get(name).cloned().unwrap_or_default();
        let fields = Arc::new(fields);
        registry.register(
            disc,
            name,
            Arc::new(move |body: &[u8]| decode(body, &fields)),
        );
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_decodes_events_from_an_idl() {
        // A minimal IDL in the modern Anchor shape: events carry name +
        // discriminator, fields live in `types`.
        let idl = r#"{
            "events": [
                { "name": "Moved", "discriminator": [1,2,3,4,5,6,7,8] }
            ],
            "types": [
                { "name": "Moved", "type": { "kind": "struct", "fields": [
                    { "name": "who", "type": "pubkey" },
                    { "name": "amount", "type": "u64" }
                ]}}
            ]
        }"#;

        let mut reg = EventRegistry::new();
        assert_eq!(register_all(&mut reg, idl), 1);

        // Build the on-wire payload: discriminator ++ borsh(who: Pubkey, amount: u64).
        let who = Pubkey::new_unique();
        let mut raw = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        raw.extend_from_slice(who.as_ref());
        raw.extend_from_slice(&100u64.to_le_bytes());

        // `decode_bytes` takes the decoded wire bytes; the base64 framing of a
        // `Program data:` line is stripped upstream by the renderer.
        let info = reg.decode_bytes(&raw).expect("decodes from idl");
        assert_eq!(info.name, "Moved");
        assert_eq!(
            info.fields,
            vec![
                ("who".to_string(), who.to_string()),
                ("amount".to_string(), "100".to_string()),
            ]
        );
    }

    #[test]
    fn an_event_with_an_unmodelled_field_falls_back_to_raw() {
        // `position` is a `defined` type the decoder doesn't model -> the whole
        // event is un-decodable (registered, but `decode` returns None).
        let idl = r#"{
            "events": [ { "name": "Opened", "discriminator": [9,9,9,9,9,9,9,9] } ],
            "types": [ { "name": "Opened", "type": { "kind": "struct", "fields": [
                { "name": "position", "type": { "defined": "Position" } }
            ]}}]
        }"#;
        let mut reg = EventRegistry::new();
        register_all(&mut reg, idl);
        assert!(reg.decode_bytes(&[9u8; 16]).is_none());
    }
}
