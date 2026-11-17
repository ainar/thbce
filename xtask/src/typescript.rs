// SPDX-License-Identifier: GPL-3.0-or-later
//! Définitions TypeScript du module WebAssembly, générées depuis les JSON Schema d'entrée et de
//! résultats (`thbce schema`, `thbce schema --resultats`).
//!
//! Le générateur couvre le sous-ensemble de JSON Schema produit par `schemars` pour les types du
//! moteur : références `$defs`, objets (`properties`, `required`, `additionalProperties`),
//! listes, `oneOf`/`anyOf`, `enum`, `const` et types simples. Toute autre construction est
//! signalée par une erreur plutôt que traduite approximativement.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Map, Value};

/// Fichier produit, inclus par `crates/thbce-wasm/src/lib.rs`.
pub const SORTIE: &str = "crates/thbce-wasm/src/types.d.ts";

/// Génère les définitions des deux schémas.
pub fn generer(schema_entree: &str, schema_resultats: &str) -> Result<String, String> {
    let entree: Value = serde_json::from_str(schema_entree).map_err(|e| e.to_string())?;
    let resultats: Value = serde_json::from_str(schema_resultats).map_err(|e| e.to_string())?;
    let mut types: BTreeMap<String, Value> = BTreeMap::new();
    for schema in [&entree, &resultats] {
        let racine = schema
            .get("title")
            .and_then(Value::as_str)
            .ok_or("schéma sans titre")?;
        let mut corps = schema.as_object().cloned().ok_or("schéma non objet")?;
        corps.remove("$schema");
        corps.remove("$defs");
        corps.remove("title");
        ajouter(&mut types, racine, Value::Object(corps))?;
        if let Some(defs) = schema.get("$defs").and_then(Value::as_object) {
            for (nom, def) in defs {
                ajouter(&mut types, nom, def.clone())?;
            }
        }
    }
    let mut s = String::from(
        "// SPDX-License-Identifier: GPL-3.0-or-later\n\
         // Fichier généré par `cargo run -p xtask -- typescript` depuis les JSON Schema d'entrée\n\
         // et de résultats ; ne pas modifier à la main.\n",
    );
    for (nom, def) in &types {
        s.push('\n');
        commentaire(&mut s, def, "");
        let corps = def
            .as_object()
            .ok_or_else(|| format!("{nom} : définition non objet"))?;
        if est_interface(corps) {
            let _ = writeln!(s, "export interface {nom} {}", objet(corps, "", nom)?);
        } else {
            let _ = writeln!(s, "export type {nom} = {};", type_ts(def, "", nom)?);
        }
    }
    Ok(s)
}

/// Ajoute une définition ; un nom présent dans les deux schémas doit y avoir la même définition.
fn ajouter(types: &mut BTreeMap<String, Value>, nom: &str, def: Value) -> Result<(), String> {
    match types.get(nom) {
        Some(existant) if *existant != def => Err(format!(
            "le type {nom} est défini différemment dans les schémas d'entrée et de résultats"
        )),
        Some(_) => Ok(()),
        None => {
            types.insert(nom.to_owned(), def);
            Ok(())
        }
    }
}

fn est_interface(o: &Map<String, Value>) -> bool {
    o.get("type").and_then(Value::as_str) == Some("object")
        && o.contains_key("properties")
        && !o.contains_key("oneOf")
        && !o.contains_key("anyOf")
}

fn commentaire(s: &mut String, def: &Value, retrait: &str) {
    let Some(d) = def.get("description").and_then(Value::as_str) else {
        return;
    };
    let d = d.replace("*/", "*\\/");
    let lignes: Vec<&str> = d.lines().collect();
    if lignes.len() == 1 {
        let _ = writeln!(s, "{retrait}/** {} */", lignes[0]);
    } else {
        let _ = writeln!(s, "{retrait}/**");
        for l in lignes {
            if l.is_empty() {
                let _ = writeln!(s, "{retrait} *");
            } else {
                let _ = writeln!(s, "{retrait} * {l}");
            }
        }
        let _ = writeln!(s, "{retrait} */");
    }
}

/// Objet à propriétés : `{ … }` sur plusieurs lignes.
fn objet(o: &Map<String, Value>, retrait: &str, ou: &str) -> Result<String, String> {
    let requis: Vec<&str> = o
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let interieur = format!("{retrait}  ");
    let mut s = String::from("{\n");
    if let Some(props) = o.get("properties").and_then(Value::as_object) {
        for (cle, p) in props {
            commentaire(&mut s, p, &interieur);
            let nom = if cle.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                cle.clone()
            } else {
                format!("\"{cle}\"")
            };
            let option = if requis.contains(&cle.as_str()) {
                ""
            } else {
                "?"
            };
            let t = type_ts(p, &interieur, &format!("{ou}.{cle}"))?;
            let _ = writeln!(s, "{interieur}{nom}{option}: {t};");
        }
    }
    if let Some(a) = o.get("additionalProperties") {
        if a.is_object() {
            let t = type_ts(a, &interieur, &format!("{ou}.*"))?;
            let _ = writeln!(s, "{interieur}[cle: string]: {t};");
        }
    }
    s.push_str(retrait);
    s.push('}');
    Ok(s)
}

fn union(variantes: &[Value], retrait: &str, ou: &str) -> Result<String, String> {
    let mut parties: Vec<String> = Vec::new();
    for v in variantes {
        let t = type_ts(v, retrait, ou)?;
        if !parties.contains(&t) {
            parties.push(t);
        }
    }
    Ok(parties.join(" | "))
}

fn simple(t: &str, ou: &str) -> Result<String, String> {
    Ok(match t {
        "string" => "string",
        "number" | "integer" => "number",
        "boolean" => "boolean",
        "null" => "null",
        "object" => "{ [cle: string]: unknown }",
        "array" => "unknown[]",
        autre => return Err(format!("{ou} : type JSON Schema inconnu « {autre} »")),
    }
    .to_owned())
}

/// Traduit un schéma en type TypeScript.
fn type_ts(schema: &Value, retrait: &str, ou: &str) -> Result<String, String> {
    let o = match schema {
        Value::Bool(true) => return Ok("unknown".into()),
        Value::Object(o) => o,
        _ => return Err(format!("{ou} : schéma inattendu")),
    };
    if let Some(r) = o.get("$ref").and_then(Value::as_str) {
        return r
            .strip_prefix("#/$defs/")
            .map(str::to_owned)
            .ok_or_else(|| format!("{ou} : référence non prise en charge {r}"));
    }
    if let Some(c) = o.get("const") {
        return Ok(c.to_string());
    }
    if let Some(e) = o.get("enum").and_then(Value::as_array) {
        let valeurs: Vec<String> = e.iter().map(Value::to_string).collect();
        return Ok(valeurs.join(" | "));
    }
    for cle in ["oneOf", "anyOf"] {
        if let Some(v) = o.get(cle).and_then(Value::as_array) {
            return union(v, retrait, ou);
        }
    }
    match o.get("type") {
        Some(Value::Array(types)) => {
            let mut parties = Vec::new();
            for t in types {
                let t = t
                    .as_str()
                    .ok_or_else(|| format!("{ou} : type non textuel"))?;
                let mut seul = o.clone();
                seul.insert("type".into(), Value::String(t.into()));
                parties.push(type_ts(&Value::Object(seul), retrait, ou)?);
            }
            Ok(parties.join(" | "))
        }
        Some(Value::String(t)) if t == "object" => {
            if o.contains_key("properties") {
                objet(o, retrait, ou)
            } else if let Some(a) = o.get("additionalProperties").filter(|a| a.is_object()) {
                Ok(format!(
                    "{{ [cle: string]: {} }}",
                    type_ts(a, retrait, &format!("{ou}.*"))?
                ))
            } else {
                simple("object", ou)
            }
        }
        Some(Value::String(t)) if t == "array" => match o.get("items") {
            Some(items) => {
                let t = type_ts(items, retrait, &format!("{ou}[]"))?;
                if t.contains(' ') && !t.starts_with('{') {
                    Ok(format!("({t})[]"))
                } else {
                    Ok(format!("{t}[]"))
                }
            }
            None => simple("array", ou),
        },
        Some(Value::String(t)) => simple(t, ou),
        None if o.keys().all(|k| k == "description" || k == "default") => Ok("unknown".into()),
        _ => Err(format!(
            "{ou} : construction JSON Schema non prise en charge"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traduit_les_constructions_du_schema() {
        let entree = r##"{
            "$schema": "x", "title": "Projet", "type": "object",
            "properties": {
                "nom": {"type": "string", "description": "Nom."},
                "n": {"type": ["integer", "null"]},
                "l": {"type": "array", "items": {"$ref": "#/$defs/Mode"}},
                "u": {"anyOf": [{"$ref": "#/$defs/Mode"}, {"type": "null"}]}
            },
            "required": ["nom", "l"],
            "$defs": {"Mode": {"oneOf": [
                {"type": "string", "enum": ["a", "b"]},
                {"type": "object", "properties": {"c": {"type": "number"}}, "required": ["c"]}
            ]}}
        }"##;
        let resultats = r#"{"title": "Resultats", "type": "object", "properties": {}}"#;
        let s = generer(entree, resultats).unwrap();
        assert!(s.contains("export interface Projet {"), "{s}");
        assert!(s.contains("  /** Nom. */\n  nom: string;"), "{s}");
        assert!(s.contains("  n?: number | null;"), "{s}");
        assert!(s.contains("  l: Mode[];"), "{s}");
        assert!(s.contains("  u?: Mode | null;"), "{s}");
        assert!(
            s.contains("export type Mode = \"a\" | \"b\" | {\n  c: number;\n};"),
            "{s}"
        );
    }
}
