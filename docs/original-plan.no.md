# Fjord — lokal prosjektstyring for Linux

> Arbeidsnavn. Open source, local-first, med GUI for mennesker og CLI/MCP for AI-agenter.

## 1. Visjon

Et raskt, vakkert skrivebordsprogram for å styre prosjekter: opprette prosjekter, oppgaver, statuser, filer og notater, alt lagret lokalt i én SQLite-database. Ingen konto, ingen sky, ingen tracking.

**Det som gjør Fjord unikt:** Det er bygget for at både du og AI-agenten din (Claude) kan jobbe i samme prosjekter. Agenten bruker en innebygd MCP-server og CLI, så den kan opprette oppgaver, oppdatere status og legge ved filer, og du ser det live i GUI-et.

### Eksisterende alternativer (og hvorfor Fjord likevel)
| Verktøy | Svakhet for vårt bruk |
|---|---|
| Plane, Vikunja, Taiga, OpenProject | Web/selvhostet server, ikke et lett skrivebordsprogram |
| Planify | Ren oppgaveliste (GTK), ikke prosjekter + filer |
| Focalboard | Ikke lenger aktivt vedlikeholdt |
| Notion/Linear/Trello | Lukket kildekode, sky, ikke lokalt |

Fjord tar nisjen **lokal, desktop, prosjekter + filer + AI-agent-API**.

## 2. Teknologivalg

| Lag | Valg | Hvorfor |
|---|---|---|
| App-rammeverk | **Tauri 2** | Lett (~10 MB mot ~150 MB med Electron), rask, sikker. Rust-backend. |
| Frontend | **React + TypeScript + Vite** | Det du kan best, så rask utvikling |
| Styling/UI | Tailwind + Radix UI + Motion | Pent, tilgjengelig, smooth animasjoner |
| Database | **SQLite** (via `sqlx` i Rust) + FTS5 | Én fil, null oppsett, fulltekstsøk |
| Filer | Innholdsadressert lagring (`~/.local/share/fjord/files/<sha256>`) | Ingen duplikater, trygg mot omdøping |
| Agent-tilgang | `fjord` CLI + `fjord mcp` (MCP-server over stdio) | Claude og andre agenter kan bruke appen |
| Pakking | AppImage, Flatpak (Flathub), .deb, AUR | Når flest mulig Linux-brukere |
| Lisens | MIT/Apache-2.0 (dobbel) | Standard i Rust-verdenen, lett å bidra til |

**Alternativ vurdert:** GTK4/libadwaita i Rust. Mer «native GNOME», men tregere å utvikle i og kjent for færre bidragsytere. Electron har mer økosystem, men er tungt. Tauri er best balanse.

## 3. Arkitektur

```
┌──────────────────────────────────────────────┐
│  GUI (React/TS i Tauri-webview)              │
└───────────────┬──────────────────────────────┘
                │ Tauri commands + events (live-oppdatering)
┌───────────────▼──────────────────────────────┐
│  fjord-core (Rust-bibliotek)                 │
│  • Prosjekter, oppgaver, filer, notater      │
│  • Validering, aktivitetslogg, søk           │
└──────┬─────────────────────────┬─────────────┘
       │                         │
┌──────▼──────┐         ┌────────▼────────┐
│ SQLite (WAL)│         │ Fillager (sha256)│
└─────────────┘         └─────────────────┘
       ▲
       │ samme kjerne
┌──────┴──────────────────────────────────────┐
│  fjord CLI  ·  fjord mcp (for AI-agenter)   │
└─────────────────────────────────────────────┘
```

- **Én kjerne, tre innganger** (GUI, CLI, MCP), så all logikk og validering ligger ett sted.
- SQLite i WAL-modus lar GUI og agent skrive samtidig. GUI-et lytter på endringer og oppdaterer seg live.
- Repo-struktur (Cargo workspace + pnpm):
  ```
  fjord/
  ├── crates/fjord-core/   # domene + database + fillager
  ├── crates/fjord-cli/    # CLI + MCP-server
  ├── src-tauri/           # Tauri-app (tynt lag over core)
  ├── ui/                  # React-frontend
  └── docs/
  ```

## 4. Datamodell (første versjon)

```sql
projects   (id, name, slug, description, color, icon, status, created_at, updated_at, archived_at)
statuses   (id, project_id, name, color, position, is_done)        -- egne kolonner per prosjekt
tasks      (id, project_id, status_id, title, body_md, priority, due_at, position, created_at, updated_at)
tags       (id, name, color)          task_tags (task_id, tag_id)
files      (id, sha256, original_name, mime, size, created_at)
attachments(id, file_id, project_id, task_id NULL, added_by, created_at)
notes      (id, project_id, title, body_md, updated_at)
activity   (id, project_id, task_id NULL, actor, action, payload_json, created_at)  -- actor = "jonas" | "claude"
search_fts (FTS5 over tasks.title/body, notes, filnavn)
```
- Tidsstempler lagres som ISO-8601-tekst (UTC).
- Migrasjoner med `sqlx migrate`, versjonert i repoet.
- `actor` i aktivitetsloggen viser om det var deg eller agenten som gjorde endringen.

## 5. Funksjoner

### MVP (v0.1)
- Opprette, redigere og arkivere prosjekter (farge + ikon)
- Kanban-tavle per prosjekt med drag & drop og egne statuskolonner
- Oppgaver med markdown, prioritet, frist og tagger
- Dra filer inn i appen for å legge dem ved prosjekt/oppgave, med forhåndsvisning (bilder, PDF, tekst)
- Prosjektoversikt/dashboard: fremdrift, forfalte oppgaver, siste aktivitet
- Globalt søk (FTS5)
- Mørk/lys modus
- CLI: `fjord project list`, `fjord task add`, `fjord task move`, `fjord attach`

### «Fantastisk kult» (v0.2–0.3)
- **⌘K-kommandopalett**: alt kan gjøres fra tastaturet (som Linear/Raycast)
- **MCP-server**: «Claude, lag oppgaver fra denne buggrapporten», og de dukker opp live på tavla med en liten 🤖-markør
- **Git-integrasjon**: koble et prosjekt til en lokal repo og se commits/branches. `fixes #12` i commit-melding flytter oppgaven til Ferdig.
- **Tidslinje/Gantt-visning** og kalendervisning
- **Fokusmodus**: én oppgave i fullskjerm med Pomodoro-timer
- Smooth animasjoner, konfetti når et prosjekt fullføres 🎉
- Temaer (inkl. egendefinerte), aksentfarge per prosjekt
- Systemvarsler for frister (libnotify)

### Senere (v1.0+)
- Valgfri synk mellom egne maskiner (f.eks. via en synk-mappe eller CRDT), fortsatt uten sky-konto
- Eksport/import: JSON, Markdown, CSV; import fra Trello/GitHub Issues
- Plugin-system
- i18n (norsk + engelsk fra start, flere via bidrag)

## 6. Agent-integrasjon (slik jeg skal bruke den)

MCP-verktøy som `fjord mcp` eksponerer:
| Verktøy | Gjør |
|---|---|
| `list_projects`, `get_project` | Lese prosjekter og status |
| `create_task`, `update_task`, `move_task` | Lage/oppdatere/flytte oppgaver |
| `attach_file` | Legge ved en fil fra disk |
| `add_note`, `search` | Notater og søk |
| `project_summary` | Kort statusrapport (brukes f.eks. i morgenmeldingen) |

Sikkerhet: agenten kan aldri slette permanent (kun arkivere), og alt logges med `actor = "claude"`, så du kan angre i GUI-et.

## 7. Faseplan

| Fase | Innhold | Estimat* |
|---|---|---|
| 0. Oppsett | Repo, Cargo/pnpm workspace, CI (GitHub Actions: lint, test, build), lisens, README | 1 helg |
| 1. Kjerne | `fjord-core`: schema, migrasjoner, CRUD, fillager, tester (TDD, 80 %+) | 1–2 uker |
| 2. GUI MVP | Prosjektliste, kanban, oppgavedetaljer, filopplasting, dashboard | 2–3 uker |
| 3. CLI + MCP | `fjord` CLI og MCP-server, så jeg kan bruke den | 1 uke |
| 4. Polish | Kommandopalett, søk, temaer, animasjoner, onboarding | 1–2 uker |
| 5. Release v0.1 | AppImage + AUR + Flatpak, nettside/README med GIF-er, Show HN / r/linux | 1 uke |

*Kveldsarbeid ved siden av jobb.

## 8. Open source-oppsett
- Offentlig GitHub-repo fra dag én: README med skjermbilder/GIF, CONTRIBUTING.md, CODE_OF_CONDUCT, issue- og PR-maler
- «good first issue»-merker for å trekke til bidragsytere
- Conventional commits + automatiske releases (release-please)
- Signerte releases og SBOM for tillit

## 9. Risiko
| Risiko | Tiltak |
|---|---|
| Tauri/webkit2gtk-forskjeller mellom distroer | Test i CI på Ubuntu + Fedora + Arch. Flatpak gir stabil runtime. |
| Store filer gjør databasen treg | Filer ligger på disk, bare metadata i SQLite |
| Samtidig skriving fra GUI og agent | WAL-modus + korte transaksjoner + endrings-events |
| Scope creep | Hold MVP stramt, alt «kult» kommer etter v0.1 |

## 10. Neste steg
1. Bestemme navn (sjekk at det er ledig på GitHub/Flathub/AUR)
2. Jeg setter opp repo-skjelett (fase 0) i `~/Documents/Prog/fjord`
3. Lage enkle skisser av hovedskjermene (prosjektliste, kanban, oppgave)
4. Starte på `fjord-core` med TDD
