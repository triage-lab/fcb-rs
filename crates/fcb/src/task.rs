//! Embedded task spec and the answer-safety invariant.
//!
//! A challenge bundle embeds the *prompts and structure* of the assignment in
//! `header.meta.task` — but never the answers. The student client decrypts the
//! whole bundle, so anything in it is visible to the student. The typed model
//! here simply has no field to hold an answer/rubric/solution, so decoding a
//! bundle strips any such field that may have leaked in. [`contains_answer_fields`]
//! is a defense-in-depth check the consumer can assert on what it decoded.

use ciborium::value::Value;
use serde::{Deserialize, Serialize};

use crate::cbor;
use crate::error::Result;

/// Map keys that must never appear in a student-facing task spec.
pub const FORBIDDEN_ANSWER_KEYS: &[&str] =
    &["answer", "answer_key", "rubric", "solution", "expected"];

/// How the student writes up the investigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportMode {
    /// A sequence of prompts answered one by one.
    Steps,
    /// A single open-ended report.
    Freeform,
}

/// One step prompt. Deliberately has no answer/expected-value field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: String,
    pub prompt: String,
    pub answer_type: String,
}

/// The embedded assignment definition (student-facing; answer-free).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSpec {
    pub report_mode: ReportMode,
    pub instructions: String,
    #[serde(default)]
    pub steps: Vec<TaskStep>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TaskMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    task: Option<TaskSpec>,
}

/// Encode a task spec into the opaque `header.meta` value.
pub fn task_to_meta(task: &TaskSpec) -> Result<Value> {
    cbor::to_value(&TaskMeta {
        task: Some(task.clone()),
    })
}

/// Read the task spec from an opaque `header.meta` value (tolerant of other
/// keys such as `streams`).
pub fn task_from_meta(meta: &Value) -> Result<Option<TaskSpec>> {
    let m: TaskMeta = cbor::from_value(meta)?;
    Ok(m.task)
}

/// Recursively report whether any forbidden answer key appears in a CBOR value.
/// Used to assert that a decoded student-facing task is answer-free.
pub fn contains_answer_fields(v: &Value) -> bool {
    match v {
        Value::Map(entries) => entries.iter().any(|(k, val)| {
            let key_is_forbidden = matches!(
                k,
                Value::Text(t) if FORBIDDEN_ANSWER_KEYS.contains(&t.as_str())
            );
            key_is_forbidden || contains_answer_fields(val)
        }),
        Value::Array(items) => items.iter().any(contains_answer_fields),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Value {
        Value::Text(s.into())
    }

    #[test]
    fn report_mode_round_trips() {
        for mode in [ReportMode::Steps, ReportMode::Freeform] {
            let spec = TaskSpec {
                report_mode: mode.clone(),
                instructions: "investigate".into(),
                steps: vec![],
            };
            let v = cbor::to_value(&spec).unwrap();
            let back: TaskSpec = cbor::from_value(&v).unwrap();
            assert_eq!(back.report_mode, mode);
        }
    }

    #[test]
    fn answer_fields_are_stripped_on_decode() {
        // A "dirty" student build whose steps wrongly carry an `answer` field.
        let dirty = Value::Map(vec![
            (t("report_mode"), t("steps")),
            (t("instructions"), t("Investigate the host.")),
            (
                t("steps"),
                Value::Array(vec![Value::Map(vec![
                    (t("id"), t("q1")),
                    (t("prompt"), t("source IP of initial intrusion?")),
                    (t("answer_type"), t("ip")),
                    (t("answer"), t("10.0.0.5")), // forbidden — must not survive
                ])]),
            ),
        ]);
        // The fixture really is dirty.
        assert!(contains_answer_fields(&dirty));

        // Decoding through the typed model has nowhere to put `answer`.
        let task: TaskSpec = cbor::from_value(&dirty).unwrap();
        assert_eq!(task.steps[0].id, "q1");
        assert_eq!(task.steps[0].answer_type, "ip");

        // Re-encoding the decoded task contains no answer field anywhere.
        let clean = cbor::to_value(&task).unwrap();
        assert!(!contains_answer_fields(&clean));
    }

    #[test]
    fn task_survives_meta_round_trip() {
        let spec = TaskSpec {
            report_mode: ReportMode::Steps,
            instructions: "go".into(),
            steps: vec![TaskStep {
                id: "q1".into(),
                prompt: "p".into(),
                answer_type: "text".into(),
            }],
        };
        let meta = task_to_meta(&spec).unwrap();
        assert_eq!(task_from_meta(&meta).unwrap(), Some(spec));
    }
}
