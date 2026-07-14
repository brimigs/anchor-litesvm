//! Namespacing pass for IDL type names that collide with the `anchor_lang`
//! prelude.
//!
//! `declare_program!` glob-imports both the `anchor_lang` prelude and an IDL's
//! generated `types` into one scope. When an IDL embeds a type whose name
//! matches a prelude export, the glob is ambiguous and rustc rejects it. The
//! real-world case is a staking program that CPIs into mpl-core: its IDL
//! embeds mpl-core's `Key` enum (an account-discriminator), and `Key` collides
//! with `anchor_lang::Key` (the trait behind `.key()`).
//!
//! Renaming the embedded type and its references before codegen dodges the
//! clash. The type is data, so the new name changes nothing at runtime: the
//! same bytes are serialized under a different Rust identifier.

use serde_json::Value;
use std::collections::BTreeMap;

/// `anchor_lang` prelude names an embedded IDL type can shadow. Extend as new
/// collisions surface; `Key` is the one observed in the wild (mpl-core).
const PRELUDE_NAMES: &[&str] = &["Key"];

/// Rename every IDL type whose name collides with the anchor prelude, prefixing
/// it with the program's PascalCase name, and rewrite every reference to it.
///
/// Returns the sanitized IDL as pretty-printed JSON. Idempotent: a second pass
/// finds nothing left to rename. An IDL with no colliding type comes back
/// re-serialized but otherwise unchanged.
pub fn sanitize_idl(idl_json: &str) -> Result<String, serde_json::Error> {
    let mut idl: Value = serde_json::from_str(idl_json)?;
    let prefix = program_prefix(&idl);

    // Rename the colliding type definitions in `types`, recording the map.
    let mut renames: BTreeMap<String, String> = BTreeMap::new();
    if let Some(types) = idl.get_mut("types").and_then(Value::as_array_mut) {
        for t in types.iter_mut() {
            let Some(name) = t.get("name").and_then(Value::as_str) else {
                continue;
            };
            if PRELUDE_NAMES.contains(&name) {
                let renamed = format!("{prefix}{name}");
                renames.insert(name.to_string(), renamed.clone());
                t["name"] = Value::String(renamed);
            }
        }
    }

    // Rewrite `{"defined": ...}` references to the renamed types anywhere.
    if !renames.is_empty() {
        rename_defined_refs(&mut idl, &renames);
    }
    serde_json::to_string_pretty(&idl)
}

/// The program's `metadata.name`, PascalCased, used as the namespace prefix.
fn program_prefix(idl: &Value) -> String {
    let name = idl
        .get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("idl");
    pascal_case(name)
}

fn pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c == ' ')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// A type reference in the new IDL spec is `{"defined": {"name": "Key"}}`, and
/// in older emitters `{"defined": "Key"}`; rewrite both forms.
fn rename_defined_refs(v: &mut Value, renames: &BTreeMap<String, String>) {
    match v {
        Value::Object(map) => {
            if let Some(defined) = map.get_mut("defined") {
                match defined {
                    Value::String(s) => {
                        if let Some(new) = renames.get(s) {
                            *s = new.clone();
                        }
                    }
                    Value::Object(inner) => {
                        if let Some(Value::String(name)) = inner.get_mut("name") {
                            if let Some(new) = renames.get(name) {
                                *name = new.clone();
                            }
                        }
                    }
                    _ => {}
                }
            }
            for child in map.values_mut() {
                rename_defined_refs(child, renames);
            }
        }
        Value::Array(items) => {
            for item in items {
                rename_defined_refs(item, renames);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_a_prelude_colliding_type_and_its_references() {
        // `Key` appears once as a type definition and twice as a reference
        // (an instruction arg and a struct field), mirroring the staking IDL.
        let idl = r#"{
          "address": "11111111111111111111111111111111",
          "metadata": { "name": "staking", "version": "0.1.0", "spec": "0.1.0" },
          "instructions": [
            { "name": "stake", "discriminator": [0],
              "accounts": [],
              "args": [{ "name": "k", "type": { "defined": { "name": "Key" } } }] }
          ],
          "types": [
            { "name": "Key", "type": { "kind": "enum", "variants": [{ "name": "AssetV1" }] } },
            { "name": "BaseAssetV1", "type": { "kind": "struct",
              "fields": [{ "name": "key", "type": { "defined": { "name": "Key" } } }] } }
          ]
        }"#;

        let out = sanitize_idl(idl).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();

        // the definition is renamed under the program namespace
        let type_names: Vec<&str> = v["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(type_names.contains(&"StakingKey"));
        assert!(!type_names.contains(&"Key"));

        // both references now point at the renamed type
        assert_eq!(
            v["instructions"][0]["args"][0]["type"]["defined"]["name"],
            "StakingKey"
        );
        assert_eq!(
            v["types"][1]["type"]["fields"][0]["type"]["defined"]["name"],
            "StakingKey"
        );

        // idempotent: a second pass is a no-op
        assert_eq!(out, sanitize_idl(&out).unwrap());
    }

    #[test]
    fn leaves_a_clean_idl_alone() {
        let idl = r#"{
          "metadata": { "name": "vault" },
          "instructions": [],
          "types": [{ "name": "VaultState", "type": { "kind": "struct", "fields": [] } }]
        }"#;
        let out = sanitize_idl(idl).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["types"][0]["name"], "VaultState");
    }
}
