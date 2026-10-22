//! Azure DevOps teams: the teams you're a member of, the area paths a team owns (its
//! work items), and its board (columns, and the state each column means per work item
//! type). Only reads from Azure.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use fjord_core::{BoardColumn, TeamBoard};

use crate::azure::{HOST, api_error, check_org, encode, find_auth, read_json};
use crate::github::{HttpError, agent, checked};
use crate::{Result, VcsError};

const AREA_PATH_FIELD: &str = "System.AreaPath";
/// The board for the backlog level stories (or PBIs, or issues) live on.
const REQUIREMENT_BACKLOG: &str = "requirement";
const DONE_COLUMN_TYPE: &str = "outgoing";
/// Listing all teams is still a preview API.
const TEAMS_API_VERSION: &str = "7.1-preview.3";
const WORK_API_VERSION: &str = "7.1";
const MAX_TEAMS: usize = 500;

/// A team you're a member of.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AzureTeam {
    pub project: String,
    pub name: String,
}

/// An area path a team owns; with `include_children` the areas below it too.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamArea {
    pub value: String,
    #[serde(default)]
    pub include_children: bool,
}

/// Whether a work item in `area` belongs to the team that owns `areas`.
pub fn in_team_areas(area: &str, areas: &[TeamArea]) -> bool {
    let area = area.trim().to_lowercase();
    areas.iter().any(|a| {
        let owned = a.value.trim().to_lowercase();
        area == owned || (a.include_children && area.starts_with(&format!("{owned}\\")))
    })
}

#[derive(Deserialize)]
struct ApiList<T> {
    value: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiTeam {
    name: String,
    project_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiFieldRef {
    reference_name: String,
}

#[derive(Deserialize)]
struct ApiTeamFieldValues {
    field: ApiFieldRef,
    #[serde(default)]
    values: Vec<TeamArea>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiBacklog {
    name: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiBoardColumn {
    name: String,
    #[serde(default)]
    state_mappings: BTreeMap<String, String>,
    #[serde(default)]
    column_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiBoardFields {
    column_field: ApiFieldRef,
}

#[derive(Deserialize)]
struct ApiBoard {
    columns: Vec<ApiBoardColumn>,
    fields: ApiBoardFields,
}

fn get<T: serde::de::DeserializeOwned>(org: &str, url: String) -> Result<T> {
    let (auth, _) = find_auth(org).ok_or_else(|| VcsError::NoAzureToken(org.to_string()))?;
    let mut res = agent()
        .get(&url)
        .header("Accept", "application/json")
        .header("Authorization", &auth.header())
        .call()
        .map_err(HttpError::from)
        .and_then(checked)
        .map_err(|e| api_error(e, org))?;
    read_json(&mut res, org)
}

fn team_url(org: &str, project: &str, team: &str, path: &str) -> String {
    format!(
        "{HOST}/{}/{}/{}/_apis/work/{path}?api-version={WORK_API_VERSION}",
        encode(org),
        encode(project),
        encode(team),
    )
}

fn check_names(project: &str, team: &str) -> Result<()> {
    if project.trim().is_empty() || team.trim().is_empty() {
        return Err(VcsError::Azure(
            "give both the Azure project and the team".into(),
        ));
    }
    Ok(())
}

/// The teams you're a member of in `org`, by project.
pub fn my_teams(org: &str) -> Result<Vec<AzureTeam>> {
    let org = check_org(org)?;
    let url = format!(
        "{HOST}/{}/_apis/teams?$mine=true&$top={MAX_TEAMS}&api-version={TEAMS_API_VERSION}",
        encode(org)
    );
    let teams: ApiList<ApiTeam> = get(org, url)?;
    let mut teams: Vec<AzureTeam> = teams
        .value
        .into_iter()
        .map(|t| AzureTeam {
            project: t.project_name,
            name: t.name,
        })
        .collect();
    teams.sort_by(|a, b| (&a.project, &a.name).cmp(&(&b.project, &b.name)));
    Ok(teams)
}

/// The area paths a team owns.
pub fn team_areas(org: &str, project: &str, team: &str) -> Result<Vec<TeamArea>> {
    let org = check_org(org)?;
    check_names(project, team)?;
    let url = team_url(
        org,
        project.trim(),
        team.trim(),
        "teamsettings/teamfieldvalues",
    );
    let values: ApiTeamFieldValues = get(org, url)?;
    if values.field.reference_name != AREA_PATH_FIELD {
        return Err(VcsError::Azure(format!(
            "the team «{team}» is organised by «{}» instead of area paths, which Fjord can't import by yet",
            values.field.reference_name
        )));
    }
    Ok(values.values)
}

/// The team's board for stories (or PBIs, or issues).
pub fn team_board(org: &str, project: &str, team: &str) -> Result<TeamBoard> {
    let org = check_org(org)?;
    check_names(project, team)?;
    let (project, team) = (project.trim(), team.trim());
    let backlogs: ApiList<ApiBacklog> = get(org, team_url(org, project, team, "backlogs"))?;
    let backlog = backlogs
        .value
        .into_iter()
        .find(|b| b.kind.eq_ignore_ascii_case(REQUIREMENT_BACKLOG))
        .ok_or_else(|| VcsError::Azure(format!("the team «{team}» has no story board")))?;
    let path = format!("boards/{}", encode(&backlog.name));
    let board: ApiBoard = get(org, team_url(org, project, team, &path))?;
    Ok(board_from(board))
}

fn board_from(board: ApiBoard) -> TeamBoard {
    TeamBoard {
        column_field: board.fields.column_field.reference_name,
        columns: board
            .columns
            .into_iter()
            .map(|c| BoardColumn {
                is_done: c.column_type.eq_ignore_ascii_case(DONE_COLUMN_TYPE),
                name: c.name,
                state_mappings: c.state_mappings,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_owned_areas_and_their_children_only_when_included() {
        let areas = [
            TeamArea {
                value: "Shop\\Payments".into(),
                include_children: true,
            },
            TeamArea {
                value: "Shop\\Web".into(),
                include_children: false,
            },
        ];
        assert!(in_team_areas("Shop\\Payments", &areas));
        assert!(in_team_areas("shop\\payments\\Refunds", &areas));
        assert!(!in_team_areas("Shop\\PaymentsLegacy", &areas));
        assert!(in_team_areas("Shop\\Web", &areas));
        assert!(!in_team_areas("Shop\\Web\\Admin", &areas));
        assert!(!in_team_areas("Shop", &areas));
    }

    #[test]
    fn reads_the_documented_board_shape() {
        let board: ApiBoard = serde_json::from_str(
            r#"{"id":"1","name":"Stories","columns":[
                {"id":"a","name":"New","itemLimit":0,"stateMappings":{"User Story":"New","Bug":"New"},"columnType":"incoming"},
                {"id":"b","name":"In Review","itemLimit":5,"stateMappings":{"User Story":"Active","Bug":"Active"},"isSplit":false,"columnType":"inProgress"},
                {"id":"c","name":"Closed","itemLimit":0,"stateMappings":{"User Story":"Closed","Bug":"Closed"},"columnType":"outgoing"}],
               "fields":{"columnField":{"referenceName":"WEF_6CB4_Kanban.Column","url":"x"},
                         "rowField":{"referenceName":"WEF_6CB4_Kanban.Lane","url":"x"},
                         "doneField":{"referenceName":"WEF_6CB4_Kanban.Column.Done","url":"x"}}}"#,
        )
        .unwrap();
        let board = board_from(board);
        assert_eq!(board.column_field, "WEF_6CB4_Kanban.Column");
        let names: Vec<&str> = board.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["New", "In Review", "Closed"]);
        assert!(board.columns[2].is_done && !board.columns[1].is_done);
        assert_eq!(board.columns[1].state_mappings["Bug"], "Active");
    }

    #[test]
    fn reads_teams_and_area_values() {
        let teams: ApiList<ApiTeam> = serde_json::from_str(
            r#"{"value":[{"id":"t1","name":"Payments","projectName":"Shop","projectId":"p"}],"count":1}"#,
        )
        .unwrap();
        assert_eq!(teams.value[0].project_name, "Shop");
        let values: ApiTeamFieldValues = serde_json::from_str(
            r#"{"field":{"referenceName":"System.AreaPath","url":"x"},"defaultValue":"Shop\\Payments",
                "values":[{"value":"Shop\\Payments","includeChildren":true}]}"#,
        )
        .unwrap();
        assert!(values.values[0].include_children);
    }
}
