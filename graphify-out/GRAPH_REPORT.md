# Graph Report - fjord  (2026-11-22)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 2430 nodes · 6460 edges · 109 communities (97 shown, 12 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 251 edges (avg confidence: 0.84)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Community 0
- Community 1
- Community 2
- Community 3
- Community 4
- Community 5
- Community 6
- Community 7
- Community 8
- Community 9
- Community 10
- Community 11
- Community 12
- Community 13
- Community 14
- Community 15
- Community 16
- Community 17
- Community 18
- Community 19
- Community 20
- Community 21
- Community 22
- Community 23
- Community 24
- Community 25
- Community 26
- Community 27
- Community 28
- Community 29
- Community 30
- Community 31
- Community 32
- Community 33
- Community 34
- Community 35
- Community 36
- Community 37
- Community 38
- Community 39
- Community 40
- Community 41
- Community 42
- Community 43
- Community 44
- Community 45
- Community 46
- Community 47
- Community 48
- Community 49
- Community 50
- Community 51
- Community 52
- Community 53
- Community 54
- Community 55
- Community 56
- Community 57
- Community 58
- Community 59
- Community 60
- Community 61
- Community 62
- Community 63
- Community 64
- Community 65
- Community 66
- Community 67
- Community 68
- Community 69
- Community 70
- Community 71
- Community 72
- Community 73
- Community 74
- Community 75
- Community 76
- Community 77
- Community 78
- Community 79
- Community 80
- Community 81
- Community 82
- Community 83
- Community 84
- Community 85
- Community 86
- Community 87
- Community 88
- Community 89
- Community 90
- Community 91
- Community 92
- Community 93
- Community 94
- Community 95
- Community 96
- Community 97
- Community 98
- Community 99
- Community 100
- Community 101
- Community 102
- Community 103
- Community 104

## God Nodes (most connected - your core abstractions)
1. `useApp()` - 120 edges
2. `AppState` - 95 edges
3. `Store` - 72 edges
4. `with_store()` - 60 edges
5. `CmdResult` - 58 edges
6. `Result` - 55 edges
7. `useLive()` - 52 edges
8. `react` - 46 edges
9. `CmdResult` - 45 edges
10. `App()` - 37 edges

## Surprising Connections (you probably didn't know these)
- `set_task_pending()` --calls--> `mark_pending()`  [INFERRED]
  src-tauri/src/git_commands.rs → crates/fjord-vcs/src/pending.rs
- `set_task_pending()` --calls--> `push_state()`  [INFERRED]
  src-tauri/src/git_commands.rs → crates/fjord-vcs/src/pending.rs
- `set_task_pending()` --calls--> `record_pushed()`  [INFERRED]
  src-tauri/src/git_commands.rs → crates/fjord-vcs/src/pending.rs
- `azure_my_boards()` --calls--> `my_boards()`  [INFERRED]
  src-tauri/src/azure_sync.rs → crates/fjord-vcs/src/azure_teams.rs
- `azure_state_columns()` --calls--> `add_board_columns()`  [INFERRED]
  src-tauri/src/azure_sync.rs → crates/fjord-vcs/src/azure_states.rs

## Import Cycles
- 1-file cycle: `crates/fjord-core/src/error.rs -> crates/fjord-core/src/error.rs`
- 2-file cycle: `crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/lib.rs`
- 2-file cycle: `crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/new_repo.rs -> crates/fjord-vcs/src/lib.rs`
- 2-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/azure.rs`
- 2-file cycle: `crates/fjord-vcs/src/git.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/git.rs`
- 2-file cycle: `crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/github.rs`
- 3-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/azure.rs`
- 3-file cycle: `crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/github.rs`
- 3-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/azure.rs`
- 3-file cycle: `crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/new_repo.rs -> crates/fjord-vcs/src/github.rs`
- 3-file cycle: `crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/credentials.rs`
- 3-file cycle: `crates/fjord-vcs/src/git.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/new_repo.rs -> crates/fjord-vcs/src/git.rs`
- 3-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/azure.rs`
- 4-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/azure.rs`
- 4-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/azure.rs`
- 4-file cycle: `crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/credentials.rs`
- 4-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/azure.rs`
- 4-file cycle: `crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/new_repo.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/credentials.rs`
- 5-file cycle: `crates/fjord-vcs/src/azure.rs -> crates/fjord-vcs/src/github.rs -> crates/fjord-vcs/src/credentials.rs -> crates/fjord-vcs/src/lib.rs -> crates/fjord-vcs/src/waiting.rs -> crates/fjord-vcs/src/azure.rs`

## Communities (109 total, 12 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.05
Nodes (114): ProjectPatch, StatusPatch, actor(), add_note(), analysis_split_sections(), analysis_template(), archive_project(), archive_task() (+106 more)

### Community 1 - "Community 1"
Cohesion: 0.05
Nodes (61): GitHubMention, LinkedPullRequest, Mention, PullRequest, azureScope(), EntryItem(), githubMentions(), linkedRepos() (+53 more)

### Community 2 - "Community 2"
Cohesion: 0.08
Nodes (62): @dnd-kit/core, @dnd-kit/sortable, @dnd-kit/utilities, Board, isOverdue(), ProjectSummary, relativeTime(), todayIso() (+54 more)

### Community 3 - "Community 3"
Cohesion: 0.06
Nodes (44): Branch, branch_name_validation(), BRANCH_TYPES, command(), Commit, FIELD_SEP, finish(), FIX_WORDS (+36 more)

### Community 4 - "Community 4"
Cohesion: 0.07
Nodes (47): @testing-library/react, vitest, Activity, formatBytes(), HelpSheet(), PRETTY, WhichKey(), AppContext (+39 more)

### Community 5 - "Community 5"
Cohesion: 0.08
Nodes (48): AzureAccount, ColumnTemplate, DeviceLogin, errorMessage(), GitHubAccount, NewProject, NewRepo, RepoOwner (+40 more)

### Community 6 - "Community 6"
Cohesion: 0.08
Nodes (50): a_failing_source_is_an_error_only_when_nothing_answered(), active_pulls(), AzBuild, AzIdentity, AzProject, AzPull, AzRepo, AzReviewer (+42 more)

### Community 7 - "Community 7"
Cohesion: 0.08
Nodes (42): Checks, GitBranch, ProjectRepo, remoteOf(), BRANCH_PREVIEW, matchBranches(), visibleBranches(), EditorSettings() (+34 more)

### Community 8 - "Community 8"
Cohesion: 0.08
Nodes (30): acceptance_and_risk_placeholders_follow_the_setting(), ANALYSIS_FOLDER, analysis_title(), AnalysisContext, creates_a_linked_note_in_the_analysis_folder_once(), custom_template_and_reset(), DEFAULT_ANALYSIS_TEMPLATE, EMPTY_CHECKLIST (+22 more)

### Community 9 - "Community 9"
Cohesion: 0.08
Nodes (16): done_tasks_report_done_and_suggestions_cover_all_kinds(), free_notes_live_outside_projects_and_can_move(), MAX_SUGGESTIONS, nested_folders_rename_and_lock(), normalize_folder(), parse_links(), parses_wiki_links(), Store (+8 more)

### Community 10 - "Community 10"
Cohesion: 0.14
Nodes (22): archived_tasks_leave_the_board(), cannot_move_to_status_of_other_project(), due_tasks_skip_done_archived_and_later_ones(), invalid_input_is_rejected(), move_before_places_task_between_neighbours(), move_task_to_done_counts_as_done(), new_task_lands_last_in_first_column_with_actor(), overdue_open_tasks_are_counted() (+14 more)

### Community 11 - "Community 11"
Cohesion: 0.10
Nodes (6): describe(), print_json(), Printer, PRIORITY, progress_bar(), task_line()

### Community 12 - "Community 12"
Cohesion: 0.08
Nodes (21): agents_see_unanalyzed_imports_until_marked(), DEFAULT_CLOSED_DAYS, DEFAULT_PENDING_STATE, default_pending_state(), deleting_a_project_drops_its_azure_mappings_and_board(), ExternalItem, ExternalLink, IMPORT_SETTINGS_KEY (+13 more)

### Community 13 - "Community 13"
Cohesion: 0.07
Nodes (40): ALL_FIELDS, ApiBuild, ApiComment, ApiComments, ApiCommitRef, ApiConnection, ApiConnectionUser, ApiIdentity (+32 more)

### Community 14 - "Community 14"
Cohesion: 0.11
Nodes (31): api_error(), ApiWorkItem, board_columns(), fetch_work_items(), field_str(), forget_az_token(), List, assigned_to() (+23 more)

### Community 15 - "Community 15"
Cohesion: 0.09
Nodes (32): is_built_in_risk(), is_risk_name(), maps_pull_requests_from_the_documented_shape(), ACCEPTANCE_FIELD, ApiField, ApiFields, apply_update(), EMPTY_LINES (+24 more)

### Community 16 - "Community 16"
Cohesion: 0.12
Nodes (34): loadMermaid(), MermaidDiagram(), CodeBlock(), Swatch(), ThemeEditor(), ThemeSettings(), activeThemeId(), applyThemeDef() (+26 more)

### Community 17 - "Community 17"
Cohesion: 0.09
Nodes (32): @tauri-apps/api, SearchHit, GLOBAL_ACTIONS, Tab, TABS, View, CommandPalette(), fuzzy() (+24 more)

### Community 18 - "Community 18"
Cohesion: 0.12
Nodes (29): lucide-react, react, react-markdown, api, Attachment, LinkTarget, Note, Status (+21 more)

### Community 19 - "Community 19"
Cohesion: 0.13
Nodes (30): describeActivity(), DueRow(), DueThisWeek(), PRIORITY_ICON, ActivityList(), Home(), useHomeKeys(), AgentTag() (+22 more)

### Community 20 - "Community 20"
Cohesion: 0.08
Nodes (30): AnalysisTarget, AttachOutcome, AzureFieldChange, AzureTeam, AzureUpdatePreview, DevicePoll, ExternalFile, ExternalLink (+22 more)

### Community 21 - "Community 21"
Cohesion: 0.11
Nodes (25): find_token(), an_analysis_note_points_at_its_azure_work_item(), analysis_target(), AnalysisTarget, download_files(), fetch_assigned_work_items(), fetch_github_mentions(), fetch_import() (+17 more)

### Community 22 - "Community 22"
Cohesion: 0.12
Nodes (20): archived_projects_are_hidden_unless_requested(), azure_columns_give_the_four_azure_states_in_english(), AZURE_STATUSES, create_project_adds_default_statuses_and_logs_activity(), DEFAULT_ICON, DEFAULT_STATUSES, duplicate_names_get_unique_slugs(), empty_name_is_rejected() (+12 more)

### Community 23 - "Community 23"
Cohesion: 0.12
Nodes (29): encode(), ApiBacklog, ApiBoard, ApiBoardColumn, ApiBoardFields, ApiFieldRef, ApiList, ApiTeam (+21 more)

### Community 24 - "Community 24"
Cohesion: 0.06
Nodes (32): author, name, url, compatibility, platforms, command, description, display_name (+24 more)

### Community 25 - "Community 25"
Cohesion: 0.14
Nodes (27): ADD, call_tool(), DEFAULT_ACTIVITY_LIMIT, error_reply(), every_tool_says_what_it_does_to_the_world(), handle_line(), handshake_lists_tools_and_ignores_notifications(), Hints (+19 more)

### Community 26 - "Community 26"
Cohesion: 0.09
Nodes (28): azure(), AzureCmd, Import, Links, Cli, column(), ColumnCmd, Add (+20 more)

### Community 27 - "Community 27"
Cohesion: 0.11
Nodes (23): ConfirmDialog(), ConfirmOptions, ContextMenu(), Dialog, MenuItem, MenuProvider(), MenusContext, MenusValue (+15 more)

### Community 28 - "Community 28"
Cohesion: 0.14
Nodes (11): attach_copies_file_and_dedupes_identical_content(), attachment_from_row(), ATTACHMENT_SELECT, attachments_can_be_filtered_by_task(), detach_keeps_blob_on_disk(), external_files_are_fetched_once_even_after_removal(), not_found_or_io(), project() (+3 more)

### Community 29 - "Community 29"
Cohesion: 0.17
Nodes (21): apply_sync(), branch_choices(), checkout_default(), fetch_pull_requests(), fetch_work_item(), Forge, Azure, GitHub (+13 more)

### Community 30 - "Community 30"
Cohesion: 0.07
Nodes (27): name, permissions, runs-on, steps, jobs, analysis, name, on (+19 more)

### Community 31 - "Community 31"
Cohesion: 0.18
Nodes (25): a_column_move_sends_exactly_one_state_for_that_work_item(), a_mapping_into_a_missing_project_is_skipped_not_fatal(), a_team_board_project_follows_board_columns_and_sends_them_back(), adding_state_columns_again_adds_nothing_and_places_existing_tasks(), apply_import(), auto_moves_leave_tasks_in_any_done_column(), azure_states_become_columns_in_order_and_tasks_follow(), column() (+17 more)

### Community 32 - "Community 32"
Cohesion: 0.08
Nodes (25): GH_TOKEN, TAG, tag, jobs, publish, name, on, release (+17 more)

### Community 33 - "Community 33"
Cohesion: 0.08
Nodes (24): Command, Attach, Azure, Column, Files, Git, Log, Mcp (+16 more)

### Community 34 - "Community 34"
Cohesion: 0.16
Nodes (20): account_for(), assigned_work_items(), Auth, Bearer, Pat, az_cli_token(), azure_account(), AzureAccount (+12 more)

### Community 35 - "Community 35"
Cohesion: 0.08
Nodes (23): app, security, windows, build, beforeBuildCommand, beforeDevCommand, devUrl, frontendDist (+15 more)

### Community 36 - "Community 36"
Cohesion: 0.15
Nodes (8): azure_my_boards(), azure_state_columns(), AZURE_WARNING_EVENT, CmdResult, Job, move_task(), queue(), send()

### Community 37 - "Community 37"
Cohesion: 0.15
Nodes (20): API_VERSION, HOST, ApiCategory, ApiList, ApiType, ApiTypeRef, AzureState, CATEGORY_ORDER (+12 more)

### Community 38 - "Community 38"
Cohesion: 0.21
Nodes (17): split_external_id(), follow_azure(), mark_pending(), PendingSync, Nothing, Push, UnknownResumeState, push_state() (+9 more)

### Community 39 - "Community 39"
Cohesion: 0.14
Nodes (16): ApiRelation, attachment_id(), attachment_ids_only_come_from_the_same_org(), ATTACHMENTS_PATH, AzureFile, collects_attachments_and_pasted_images_once(), DEFAULT_IMAGE_NAME, download_file() (+8 more)

### Community 40 - "Community 40"
Cohesion: 0.16
Nodes (8): board(), board_and_item_columns_round_trip(), BOARD_KEY_PREFIX, BoardColumn, finds_columns_by_name_and_by_state(), Store, TeamBoard, column_name()

### Community 41 - "Community 41"
Cohesion: 0.20
Nodes (15): Error, Db, Invalid, Io, NotFound, Result, invalid_targets_are_rejected(), matching_status() (+7 more)

### Community 42 - "Community 42"
Cohesion: 0.15
Nodes (10): platform_dirs_end_in_fjord_and_env_overrides_win(), configure(), migrate(), MIGRATIONS, config_dir(), DB_FILE, default_actor(), env_or_platform() (+2 more)

### Community 43 - "Community 43"
Cohesion: 0.12
Nodes (18): API, ApiCheckRun, ApiCheckRuns, ApiDeviceCode, ApiRepo, ApiReview, ApiUser, combine_reviews() (+10 more)

### Community 44 - "Community 44"
Cohesion: 0.17
Nodes (6): ApiReviewer, AzureDevOps, branch(), review_from_votes(), review_state_from_azure_votes(), reviewer()

### Community 45 - "Community 45"
Cohesion: 0.13
Nodes (12): skill_cmd(), SkillCmd, Install, default_root(), install(), install_writes_and_replaces_the_skill(), SKILL_MD, SKILL_NAME (+4 more)

### Community 46 - "Community 46"
Cohesion: 0.21
Nodes (11): create_appends_column_and_validates_input(), DEFAULT_COLUMN_COLOR, delete_refuses_non_empty_and_last_column(), DONE_NAMES, move_reorders_columns(), names(), project(), sounds_done() (+3 more)

### Community 47 - "Community 47"
Cohesion: 0.10
Nodes (19): compilerOptions, allowArbitraryExtensions, allowImportingTsExtensions, erasableSyntaxOnly, jsx, lib, module, moduleDetection (+11 more)

### Community 48 - "Community 48"
Cohesion: 0.16
Nodes (8): NewTask, delete_note(), delete_project_removes_everything_inside(), delete_task_removes_it_and_unreferenced_files(), settings_round_trip_and_user_rename(), Store, USER_NAME_KEY, require_text()

### Community 49 - "Community 49"
Cohesion: 0.16
Nodes (14): CACHE, CACHE_TTL, delete(), entry(), forget(), get(), GITHUB, read() (+6 more)

### Community 50 - "Community 50"
Cohesion: 0.11
Nodes (19): canvas-confetti, motion, @tauri-apps/plugin-notification, @tauri-apps/plugin-process, @tauri-apps/plugin-updater, dependencies, canvas-confetti@^1.9.4, @dnd-kit/core@^6.3.1 (+11 more)

### Community 51 - "Community 51"
Cohesion: 0.12
Nodes (14): Activity, Attachment, ExternalFile, LinkTarget, NewProject, Note, present_or_null(), Project (+6 more)

### Community 52 - "Community 52"
Cohesion: 0.12
Nodes (15): branch_name_for_task(), branch_names_from_tasks(), branch_type(), task_id_from_branch(), add_work_item_task(), branch_for(), column_for_state(), follow_imported() (+7 more)

### Community 53 - "Community 53"
Cohesion: 0.13
Nodes (11): LABEL_PREFIX, MAIN, MainNav, Note, Project, Task, MIN_SIZE, NAV_EVENT (+3 more)

### Community 54 - "Community 54"
Cohesion: 0.20
Nodes (4): DeviceLogin, GitHub, github_form(), start_github_login()

### Community 55 - "Community 55"
Cohesion: 0.25
Nodes (11): azure_import_needs_setup_and_links_start_empty(), bad_input_fails_with_message_and_nonzero_exit(), delete_requires_confirmation_or_yes(), files_are_attached_and_listed(), fjord(), git_link_branch_and_status(), notes_folders_move_rename_pin_and_lock(), ok_json() (+3 more)

### Community 56 - "Community 56"
Cohesion: 0.12
Nodes (16): patterns, patterns, actions, cargo, npm, dependency-name, update-types, patterns (+8 more)

### Community 57 - "Community 57"
Cohesion: 0.18
Nodes (11): react-dom, MainNav, Project, closeWindow(), TaskWindow(), useFileDrop(), externalHref(), routeExternalLinks() (+3 more)

### Community 58 - "Community 58"
Cohesion: 0.18
Nodes (13): byName(), byNumber(), collator, DEFAULT_PROJECT_SORT, LastUsed, leadingNumber(), PROJECT_SORTS, ProjectSortBy (+5 more)

### Community 59 - "Community 59"
Cohesion: 0.12
Nodes (16): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, lib, module, moduleDetection, noEmit, noFallthroughCasesInSwitch (+8 more)

### Community 60 - "Community 60"
Cohesion: 0.22
Nodes (15): agent_reads_and_edits_notespace(), agent_starts_a_story_analysis_once(), agent_triages_imported_tasks(), agent_workflow_create_move_complete_search(), call(), get_task_returns_body_subtasks_and_files(), git(), git_tools_link_a_repo_and_branch_from_a_task() (+7 more)

### Community 61 - "Community 61"
Cohesion: 0.23
Nodes (7): RemoteHost, AzureDevOps, GitHub, link_relink_and_unlink_repo(), setup(), Store, task_branch_is_stored_found_and_cleared()

### Community 62 - "Community 62"
Cohesion: 0.12
Nodes (16): with, args, cache, cache-dependency-path, if-no-files-found, includeUpdaterJson, name, node-version (+8 more)

### Community 63 - "Community 63"
Cohesion: 0.20
Nodes (10): brings_back_a_window_that_opened_partly_off_screen(), FILL, fit_main_window(), fit_to_screen(), fitted(), LAPTOP, leaves_a_window_that_fits_alone(), Rect (+2 more)

### Community 64 - "Community 64"
Cohesion: 0.13
Nodes (15): jsdom, @tauri-apps/cli, @testing-library/dom, typescript, vite, @vitejs/plugin-react, devDependencies, jsdom@^30.1.3 (+7 more)

### Community 65 - "Community 65"
Cohesion: 0.14
Nodes (14): TaskCmd, Add, Analyze, Archive, Archived, Delete, Done, Edit (+6 more)

### Community 66 - "Community 66"
Cohesion: 0.21
Nodes (13): parse_github_remote(), parses_github_remotes(), auto_move_can_be_turned_off(), detect_host(), link_branch(), link_detects_azure_devops_remotes(), link_existing_branch_to_a_task(), link_repo() (+5 more)

### Community 67 - "Community 67"
Cohesion: 0.25
Nodes (11): buildTree(), byName(), countNotes(), FolderNode, folderPaths(), NoteRow, NoteSort, renamedPath() (+3 more)

### Community 68 - "Community 68"
Cohesion: 0.23
Nodes (12): connect_github(), connect_rejects_malformed_tokens_without_calling_github(), find_token_with_source(), gh_cli_token(), github_account(), GitHubAccount, TokenSource, Cli (+4 more)

### Community 69 - "Community 69"
Cohesion: 0.17
Nodes (12): VcsError, Azure, Core, Credentials, Git, GitHub, GitMissing, NoAzureToken (+4 more)

### Community 70 - "Community 70"
Cohesion: 0.29
Nodes (9): a_path_cannot_add_arguments_or_commands(), builds_the_editor_command(), editor_command(), missing_editor_gives_a_helpful_error(), open_in_editor(), PATH_PLACEHOLDER, resolve(), split_words() (+1 more)

### Community 71 - "Community 71"
Cohesion: 0.27
Nodes (8): api_error(), checked(), errors_keep_the_services_own_explanation(), HttpError, Status, Transport, service_message(), with_message()

### Community 72 - "Community 72"
Cohesion: 0.18
Nodes (11): APPLE_CERTIFICATE, APPLE_CERTIFICATE_PASSWORD, APPLE_ID, APPLE_PASSWORD, APPLE_SIGNING_IDENTITY, APPLE_TEAM_ID, GH_TOKEN, GITHUB_TOKEN (+3 more)

### Community 73 - "Community 73"
Cohesion: 0.18
Nodes (10): description, name, packages, repository, source, url, $schema, title (+2 more)

### Community 74 - "Community 74"
Cohesion: 0.18
Nodes (10): remark-gfm, @tauri-apps/plugin-dialog, @types/react, remark-gfm@^4.0.1, @tauri-apps/plugin-dialog@^2.8.3, @types/react@^19.2.18, name, private (+2 more)

### Community 75 - "Community 75"
Cohesion: 0.20
Nodes (10): GitCmd, AutoMove, Branch, Link, Main, Pr, Prs, Status (+2 more)

### Community 76 - "Community 76"
Cohesion: 0.24
Nodes (10): ApiDeviceToken, device_flow_responses_map_to_poll_states(), device_poll_from(), DevicePoll, Denied, Done, Expired, Pending (+2 more)

### Community 77 - "Community 77"
Cohesion: 0.20
Nodes (10): name, needs, runs-on, strategy, name, os, cross-platform, include (+2 more)

### Community 78 - "Community 78"
Cohesion: 0.22
Nodes (8): folds_builds_into_checks(), Checks, Failure, None, Pending, Success, combine_checks(), combines_check_runs()

### Community 79 - "Community 79"
Cohesion: 0.25
Nodes (8): ApiPull, ApiRef, PullRequest, Review, Approved, ChangesRequested, None, Required

### Community 80 - "Community 80"
Cohesion: 0.22
Nodes (8): concurrency, cancel-in-progress, group, env, CARGO_TERM_COLOR, name, permissions, contents

### Community 81 - "Community 81"
Cohesion: 0.22
Nodes (8): name, needs, runs-on, jobs, cli, name, permissions, contents

### Community 82 - "Community 82"
Cohesion: 0.25
Nodes (8): jobs, rust, ui, name, needs, runs-on, name, runs-on

### Community 83 - "Community 83"
Cohesion: 0.25
Nodes (8): with, cache, cache-dependency-path, components, name, node-version, path, retention-days

### Community 84 - "Community 84"
Cohesion: 0.36
Nodes (8): steps, steps, continue-on-error, if, name, run, shell, uses

### Community 86 - "Community 86"
Cohesion: 0.48
Nodes (7): steps, steps, steps, name, run, uses, steps

### Community 87 - "Community 87"
Cohesion: 0.33
Nodes (7): name, runs-on, strategy, strategy, app, fail-fast, matrix

### Community 88 - "Community 88"
Cohesion: 0.29
Nodes (7): scripts, build, dev, lint, preview, test, typecheck

### Community 89 - "Community 89"
Cohesion: 0.33
Nodes (6): args, ext, name, os, target, include

### Community 90 - "Community 90"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 91 - "Community 91"
Cohesion: 0.33
Nodes (5): plugins, rules, react/only-export-components, react/rules-of-hooks, $schema

### Community 92 - "Community 92"
Cohesion: 0.40
Nodes (3): Columns, Board, States

### Community 93 - "Community 93"
Cohesion: 0.60
Nodes (5): add_board_columns(), add_state_columns(), ensure_columns(), place_imported_tasks(), StateColumnsReport

### Community 94 - "Community 94"
Cohesion: 0.40
Nodes (5): on, pull_request, push, workflow_dispatch, branches

### Community 95 - "Community 95"
Cohesion: 0.50
Nodes (4): name, needs, runs-on, bundle

### Community 96 - "Community 96"
Cohesion: 0.50
Nodes (4): on, push, workflow_dispatch, tags

### Community 97 - "Community 97"
Cohesion: 0.83
Nodes (4): fjord-app, fjord-cli, fjord-core, fjord-vcs

## Knowledge Gaps
- **108 isolated node(s):** `Created`, `Unchanged`, `Updated`, `Import`, `Links` (+103 more)
  These have ≤1 connection - possible missing edges. (Counts symbols only; 682 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **12 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Store` connect `Community 42` to `Community 0`, `Community 8`, `Community 10`, `Community 12`, `Community 21`, `Community 25`, `Community 26`, `Community 29`, `Community 31`, `Community 33`, `Community 36`, `Community 37`, `Community 38`, `Community 40`, `Community 41`, `Community 52`, `Community 66`, `Community 70`, `Community 85`, `Community 93`?**
  _High betweenness centrality (0.056) - this node is a cross-community bridge._
- **What connects `Created`, `Unchanged`, `Updated` to the rest of the system?**
  _108 weakly-connected nodes found - possible missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.051110180142438205 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `Community 29` to `Community 3`, `Community 6`, `Community 11`, `Community 13`, `Community 14`, `Community 15`, `Community 21`, `Community 23`, `Community 25`, `Community 26`, `Community 31`, `Community 37`, `Community 38`, `Community 39`, `Community 43`, `Community 45`, `Community 49`, `Community 52`, `Community 66`, `Community 70`?**
  _High betweenness centrality (0.036) - this node is a cross-community bridge._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.05126452494873548 - nodes in this community are weakly interconnected._
- **Why does `AppState` connect `Community 0` to `Community 42`, `Community 36`, `Community 85`?**
  _High betweenness centrality (0.022) - this node is a cross-community bridge._
- **Should `Community 2` be split into smaller, more focused modules?**
  _Cohesion score 0.07942097026604068 - nodes in this community are weakly interconnected._