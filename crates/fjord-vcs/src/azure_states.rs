//! Azure Boards states as board columns: reads the states of an Azure DevOps
//! project's work item types (New, Active, Resolved, Closed, …) and gives a Fjord
//! project a column for each, in Azure's order. Only reads from Azure.

use serde::{Deserialize, Serialize};

use fjord_core::{Store, TeamBoard};

use crate::azure::{API_VERSION, HOST, api_error, check_org, encode, find_auth, read_json};
use crate::github::{HttpError, agent, checked};
use crate::{Result, VcsError, split_external_id};

/// The work item types people plan with; test plans, code reviews and the like are left out.
const PLANNING_CATEGORIES: &[&str] = &[
    "Microsoft.RequirementCategory",
    "Microsoft.EpicCategory",
    "Microsoft.FeatureCategory",
    "Microsoft.BugCategory",
    "Microsoft.TaskCategory",
];
/// Azure's state categories, in board order. "Removed" isn't a column.
const CATEGORY_ORDER: &[&str] = &["Proposed", "InProgress", "Resolved", "Completed"];
const REMOVED: &str = "Removed";
const COMPLETED: &str = "Completed";

/// One state a work item can be in.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AzureState {
    pub name: String,
    /// Proposed, InProgress, Resolved, Completed or Removed.
    #[serde(default)]
    pub category: String,
}

#[derive(Deserialize)]
struct ApiList<T> {
    value: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiCategory {
    reference_name: String,
    #[serde(default)]
    work_item_types: Vec<ApiTypeRef>,
}

#[derive(Deserialize)]
struct ApiTypeRef {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiType {
    name: String,
    #[serde(default)]
    is_disabled: bool,
    #[serde(default)]
    states: Vec<AzureState>,
}

/// What adding the columns did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct StateColumnsReport {
    /// Columns created, in board order.
    pub added: Vec<String>,
    /// Imported tasks moved into the column for their work item's state.
    pub placed: usize,
}

fn get<T: serde::de::DeserializeOwned>(org: &str, project: &str, path: &str) -> Result<T> {
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let mut res = agent()
        .get(&format!(
            "{HOST}/{}/{}/_apis/wit/{path}?api-version={API_VERSION}",
            encode(org),
            encode(project)
        ))
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| api_error(e, org))?;
    read_json(&mut res, org)
}

/// The states of the project's planning work item types, merged in board order.
pub fn project_states(org: &str, project: &str) -> Result<Vec<AzureState>> {
    let org = check_org(org)?;
    let project = project.trim();
    if project.is_empty() {
        return Err(VcsError::Azure("the Azure project name is empty".into()));
    }
    let categories: ApiList<ApiCategory> = get(org, project, "workitemtypecategories")?;
    let mut types: ApiList<ApiType> = get(org, project, "workitemtypes")?;
    let planning = planning_types(&categories.value);
    types.value.retain(|t| {
        !t.is_disabled
            && (planning.is_empty() || planning.iter().any(|n| n.eq_ignore_ascii_case(&t.name)))
    });
    types.value.sort_by_key(|t| {
        planning
            .iter()
            .position(|n| n.eq_ignore_ascii_case(&t.name))
    });
    // Older servers leave the states out of the list; ask per type then.
    for t in types.value.iter_mut().filter(|t| t.states.is_empty()) {
        let path = format!("workitemtypes/{}/states", encode(&t.name));
        t.states = get::<ApiList<AzureState>>(org, project, &path)?.value;
    }
    let states: Vec<&[AzureState]> = types.value.iter().map(|t| t.states.as_slice()).collect();
    Ok(merge_states(&states))
}

/// Type names in the planning categories, requirements (User Story, PBI) first so
/// their states lead.
fn planning_types(categories: &[ApiCategory]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for reference in PLANNING_CATEGORIES {
        let Some(category) = categories.iter().find(|c| c.reference_name == *reference) else {
            continue;
        };
        for t in &category.work_item_types {
            if !names.iter().any(|n| n.eq_ignore_ascii_case(&t.name)) {
                names.push(t.name.clone());
            }
        }
    }
    names
}

fn category_rank(category: &str) -> usize {
    CATEGORY_ORDER
        .iter()
        .position(|c| c.eq_ignore_ascii_case(category))
        // Unknown categories sit with the in-progress ones.
        .unwrap_or(1)
}

/// Each state once (first spelling wins), grouped by category in board order and
/// otherwise in the order the types list them. Removed states are left out.
pub fn merge_states(types: &[&[AzureState]]) -> Vec<AzureState> {
    let mut merged: Vec<AzureState> = Vec::new();
    for state in types.iter().flat_map(|t| t.iter()) {
        let name = state.name.trim();
        if name.is_empty()
            || state.category.eq_ignore_ascii_case(REMOVED)
            || merged.iter().any(|m| m.name.eq_ignore_ascii_case(name))
        {
            continue;
        }
        merged.push(AzureState {
            name: name.to_string(),
            category: state.category.clone(),
        });
    }
    merged.sort_by_key(|s| category_rank(&s.category));
    merged
}

/// Store only: adds a column for each state the project doesn't have yet, puts the
/// state columns first in Azure's order (other columns keep their order after them),
/// and moves imported tasks into the column for their work item's current state.
/// The pending state is left out; pending is a flag in Fjord, not a column.
pub fn add_state_columns(
    store: &mut Store,
    project_id: i64,
    states: &[AzureState],
) -> Result<StateColumnsReport> {
    let settings = store.import_settings()?;
    let states: Vec<&AzureState> = states
        .iter()
        .filter(|s| !settings.is_pending_state(&s.name))
        .collect();
    let wanted: Vec<(&str, bool)> = states
        .iter()
        .map(|s| (s.name.as_str(), s.category.eq_ignore_ascii_case(COMPLETED)))
        .collect();
    let added = ensure_columns(store, project_id, &wanted)?;
    store.clear_team_board(project_id)?;
    let placed = place_imported_tasks(store, project_id)?;
    Ok(StateColumnsReport { added, placed })
}

/// Store only: like [`add_state_columns`] for a team's board. The project remembers the
/// board, so imports follow each work item's board column and moves send it back.
pub fn add_board_columns(
    store: &mut Store,
    project_id: i64,
    board: &TeamBoard,
) -> Result<StateColumnsReport> {
    let wanted: Vec<(&str, bool)> = board
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.is_done))
        .collect();
    let added = ensure_columns(store, project_id, &wanted)?;
    store.set_team_board(project_id, board)?;
    let placed = place_imported_tasks(store, project_id)?;
    Ok(StateColumnsReport { added, placed })
}

/// Adds the columns the project doesn't have yet and puts `wanted` first, in order;
/// other columns keep their order after them. Returns the names added.
fn ensure_columns(
    store: &mut Store,
    project_id: i64,
    wanted: &[(&str, bool)],
) -> Result<Vec<String>> {
    let mut added = Vec::new();
    for (name, done) in wanted {
        let exists = store
            .list_statuses(project_id)?
            .iter()
            .any(|c| c.name.trim().eq_ignore_ascii_case(name));
        if !exists {
            store.create_status(project_id, name, None, *done)?;
            added.push(name.to_string());
        }
    }
    let columns = store.list_statuses(project_id)?;
    let order: Vec<i64> = wanted
        .iter()
        .filter_map(|(name, _)| {
            columns
                .iter()
                .find(|c| c.name.trim().eq_ignore_ascii_case(name))
                .map(|c| c.id)
        })
        .collect();
    for (index, id) in order.into_iter().enumerate() {
        if store.list_statuses(project_id)?.get(index).map(|c| c.id) != Some(id) {
            store.move_status(id, index)?;
        }
    }
    Ok(added)
}

/// Moves the project's imported tasks into the column for their work item's board
/// column or state; tasks without a matching column stay put. Never writes to Azure.
fn place_imported_tasks(store: &mut Store, project_id: i64) -> Result<usize> {
    let columns = store.list_statuses(project_id)?;
    let board = store.team_board(project_id)?;
    let mut placed = 0;
    for (external_id, task_id) in store.external_ids("azure")? {
        if split_external_id(&external_id).is_none() {
            continue;
        }
        let task = store.get_task(task_id)?;
        if task.project_id != project_id || task.archived_at.is_some() {
            continue;
        }
        let Some(name) = column_name(store, board.as_ref(), task_id)? else {
            continue;
        };
        let target = columns
            .iter()
            .find(|c| c.name.trim().eq_ignore_ascii_case(name.trim()));
        if let Some(column) = target.filter(|c| c.id != task.status_id) {
            store.move_task(task_id, column.id, None)?;
            placed += 1;
        }
    }
    Ok(placed)
}

/// The Fjord column an imported task belongs in: on a team board its board column (or,
/// before an import has seen it, the first board column for its state); else its state.
fn column_name(store: &Store, board: Option<&TeamBoard>, task_id: i64) -> Result<Option<String>> {
    let state = store.external_state(task_id)?;
    let Some(board) = board else {
        return Ok(state);
    };
    if let Some(column) = store.external_board_column(task_id)? {
        return Ok(Some(column));
    }
    let kind = store.external_kind(task_id)?.unwrap_or_default();
    Ok(state.and_then(|s| board.column_for_state(&kind, &s).map(|c| c.name.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(name: &str, category: &str) -> AzureState {
        AzureState {
            name: name.into(),
            category: category.into(),
        }
    }

    #[test]
    fn merges_states_in_board_order_without_removed_or_duplicates() {
        let story = [
            state("New", "Proposed"),
            state("Active", "InProgress"),
            state("Resolved", "Resolved"),
            state("Closed", "Completed"),
            state("Removed", "Removed"),
        ];
        let task = [
            state("new", "Proposed"),
            state("Active", "InProgress"),
            state("Closed", "Completed"),
        ];
        let bug = [
            state("Triage", "Proposed"),
            state("Active", "InProgress"),
            state("Pending", "InProgress"),
        ];
        let names: Vec<String> = merge_states(&[&story, &task, &bug])
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(
            names,
            ["New", "Triage", "Active", "Pending", "Resolved", "Closed"]
        );
    }

    #[test]
    fn requirement_types_come_first_and_hidden_types_are_left_out() {
        let categories: ApiList<ApiCategory> = serde_json::from_str(
            r#"{"value":[
                {"referenceName":"Microsoft.TaskCategory","workItemTypes":[{"name":"Task"}]},
                {"referenceName":"Microsoft.HiddenCategory","workItemTypes":[{"name":"Code Review Request"}]},
                {"referenceName":"Microsoft.RequirementCategory","workItemTypes":[{"name":"User Story"}]},
                {"referenceName":"Microsoft.BugCategory","workItemTypes":[{"name":"Bug"}]}
            ]}"#,
        )
        .unwrap();
        assert_eq!(
            planning_types(&categories.value),
            ["User Story", "Bug", "Task"]
        );
    }

    #[test]
    fn reads_states_from_the_documented_type_shape() {
        let types: ApiList<ApiType> = serde_json::from_str(
            r#"{"value":[{"name":"Bug","isDisabled":false,"referenceName":"Microsoft.VSTS.WorkItemTypes.Bug",
                "states":[{"name":"New","color":"b2b2b2","category":"Proposed"},
                          {"name":"Closed","color":"339933","category":"Completed"}]}]}"#,
        )
        .unwrap();
        assert_eq!(
            types.value[0].states,
            [state("New", "Proposed"), state("Closed", "Completed")]
        );
    }
}
