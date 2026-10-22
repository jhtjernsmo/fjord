//! Azure DevOps teams: the boards your work items are on, the area paths a team owns
//! (its work items), and its board (columns, and the state each column means per work
//! item type). Only reads from Azure.

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
const WORK_API_VERSION: &str = "7.1";
const MAX_TEAMS: usize = 500;

/// A team whose board has work items assigned to you.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AzureTeam {
    pub project: String,
    pub name: String,
    /// How many of your open work items are on its board.
    pub tasks: usize,
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
    owning_depth(area, areas).is_some()
}

/// How deep the team's area holding `area` is (`Shop` 1, `Shop\Payments` 2), so the
/// most specific team can win when several own the same work item.
pub fn owning_depth(area: &str, areas: &[TeamArea]) -> Option<usize> {
    let area = area.trim().to_lowercase();
    areas
        .iter()
        .filter(|a| {
            let owned = a.value.trim().to_lowercase();
            area == owned || (a.include_children && area.starts_with(&format!("{owned}\\")))
        })
        .map(|a| a.value.split('\\').count())
        .max()
}

#[derive(Deserialize)]
struct ApiList<T> {
    value: Vec<T>,
}

#[derive(Deserialize)]
struct ApiTeam {
    name: String,
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

/// The teams whose boards hold your open work items, by project and then by how many,
/// whether or not you're a member of the team. Teams organised by something other than
/// area paths are left out.
pub fn my_boards(org: &str, items: &[crate::WorkItem]) -> Result<Vec<AzureTeam>> {
    let org = check_org(org)?;
    let mut projects: Vec<&str> = items.iter().map(|i| i.project.as_str()).collect();
    projects.sort_unstable();
    projects.dedup();
    let mut boards = Vec::new();
    for project in projects {
        let url = format!(
            "{HOST}/{}/_apis/projects/{}/teams?$top={MAX_TEAMS}&api-version={WORK_API_VERSION}",
            encode(org),
            encode(project)
        );
        let teams: ApiList<ApiTeam> = get(org, url)?;
        let areas: Vec<(String, Vec<TeamArea>)> = teams
            .value
            .into_iter()
            .filter_map(|t| {
                let areas = team_areas(org, project, &t.name).ok()?;
                Some((t.name, areas))
            })
            .collect();
        let in_project: Vec<&str> = items
            .iter()
            .filter(|i| i.project == project)
            .map(|i| i.area.as_str())
            .collect();
        boards.extend(count_on_boards(project, &areas, &in_project));
    }
    Ok(boards)
}

/// The teams with at least one of `item_areas` in their areas, most items first.
fn count_on_boards(
    project: &str,
    teams: &[(String, Vec<TeamArea>)],
    item_areas: &[&str],
) -> Vec<AzureTeam> {
    let mut boards: Vec<AzureTeam> = teams
        .iter()
        .map(|(name, areas)| AzureTeam {
            project: project.to_string(),
            name: name.clone(),
            tasks: item_areas
                .iter()
                .filter(|a| in_team_areas(a, areas))
                .count(),
        })
        .filter(|b| b.tasks > 0)
        .collect();
    boards.sort_by(|a, b| b.tasks.cmp(&a.tasks).then_with(|| a.name.cmp(&b.name)));
    boards
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
    fn lists_the_boards_holding_your_items_most_first() {
        let area = |value: &str, include_children: bool| TeamArea {
            value: value.into(),
            include_children,
        };
        let teams = vec![
            ("Shop".to_string(), vec![area("Shop", true)]),
            ("Payments".to_string(), vec![area("Shop\\Payments", true)]),
            ("Web".to_string(), vec![area("Shop\\Web", false)]),
            ("Mobile".to_string(), vec![area("Shop\\Mobile", true)]),
        ];
        let items = [
            "Shop\\Payments",
            "Shop\\Payments\\Refunds",
            "Shop\\Web",
            "Shop",
        ];
        let boards: Vec<(String, usize)> = count_on_boards("Shop", &teams, &items)
            .into_iter()
            .map(|b| (b.name, b.tasks))
            .collect();
        // The project's default team owns every area below it; Mobile has none of yours.
        assert_eq!(
            boards,
            [
                ("Shop".to_string(), 4),
                ("Payments".to_string(), 2),
                ("Web".to_string(), 1)
            ]
        );
    }

    #[test]
    fn reads_teams_and_area_values() {
        let teams: ApiList<ApiTeam> = serde_json::from_str(
            r#"{"value":[{"id":"t1","name":"Payments","projectName":"Shop","projectId":"p"}],"count":1}"#,
        )
        .unwrap();
        assert_eq!(teams.value[0].name, "Payments");
        let values: ApiTeamFieldValues = serde_json::from_str(
            r#"{"field":{"referenceName":"System.AreaPath","url":"x"},"defaultValue":"Shop\\Payments",
                "values":[{"value":"Shop\\Payments","includeChildren":true}]}"#,
        )
        .unwrap();
        assert!(values.values[0].include_children);
    }
}
