//! `rbit behave`: the project's behaviour checks.

use std::fmt::Write as _;

use anyhow::Result;
use serde_json::json;

use super::Outcome;
use crate::behave;
use crate::project::Loaded;

pub(super) fn behave(loaded: &Loaded, only: &[String]) -> Result<Outcome> {
    let outcomes = behave::behave(loaded, only)?;
    let agree = outcomes.iter().filter(|outcome| outcome.agree).count();
    let mut text = String::new();
    for outcome in &outcomes {
        if outcome.agree {
            writeln!(text, "agree    {}", outcome.id)?;
        } else if outcome.reference.timed_out || outcome.artifact.timed_out {
            writeln!(text, "timeout  {}", outcome.id)?;
        } else {
            writeln!(
                text,
                "differ   {} ({})",
                outcome.id,
                outcome.differs.join(", ")
            )?;
        }
    }
    write!(
        text,
        "{agree}/{} checks agree; behaviour on these inputs only, not byte identity",
        outcomes.len()
    )?;
    Ok(Outcome {
        ok: agree == outcomes.len(),
        json: json!({ "agree": agree == outcomes.len(), "checks": outcomes }),
        text,
    })
}
