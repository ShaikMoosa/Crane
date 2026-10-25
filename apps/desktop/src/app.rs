use crate::draft::{Draft, DraftCoordinator};
use crate::formatting::FormattingState;
use crate::inspector::{InspectorIntent, InspectorState};
use crate::knowledge_navigation::{
    ChooserIntent, ChooserState, CompletionState, KnowledgeNavIntent,
};
use crate::markdown_layouter::MarkdownLayouter;
use crate::query_dispatch::{enqueue_deferred_query, flush_deferred_queries, PendingQuery};
use crate::quick_open::{QuickOpenIntent, QuickOpenState};
use crate::settings::SettingsController;
use crate::sidebar::{SidebarIntent, SidebarState, SidebarView};
use crate::theme;
use crate::transfers::{TransferController, TransferEffect, TransferState};
use crate::view_state::{
    editor_measure, focus_transition_seconds, inspector_presentation, layout_mode,
    InspectorPresentation, LayoutMode, ViewState, COLLAPSED_NOTE_LIST_WIDTH, COLLAPSED_RAIL_WIDTH,
    INSPECTOR_WIDTH, NOTE_LIST_WIDTH, RAIL_WIDTH,
};
use crate::views::{editor, library, note_list, restore};
use crate::worker::{spawn_worker, WorkerHandle, WorkerOutcome, WorkerRequest, WorkerResult};
use crane_application::knowledge::{
    CollectionView, KnowledgeView, Query, QueryData, ResolutionView, SidebarCounts,
};
use crane_application::{
    AppError, Command, CommandData, ErrorCode, Library, LibrarySnapshot, NoteState,
};
use crane_domain::knowledge::{parse_draft_facts, HeadingResolution, NoteFacts, TagNode, WikiLink};
use crane_domain::{parse_preview, MarkdownDocument, Note, Store};
use crane_storage::{NativeSettings, ThemeChoice};
use eframe::egui;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

const GENERIC_REQUEST_START: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub query_recomputes: u64,
    pub preview_recomputes: u64,
    pub markdown_parses: u64,
}

type ListKey = (u64, CollectionView, Option<String>, String);

#[derive(Default)]
struct QueryCache {
    key: Option<ListKey>,
    ids: Vec<String>,
}

#[derive(Default)]
struct PreviewCache {
    key: Option<(String, u64)>,
    document: Option<MarkdownDocument>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MobilePane {
    Library,
    NoteList,
    Editor,
}

#[derive(Clone, Debug)]
struct PendingWikiJump {
    note_id: String,
    byte_offset: usize,
    anchor: String,
}

enum DeferredAction {
    Create,
    Select(String),
    Navigate {
        note_id: String,
        heading: Option<HeadingResolution>,
    },
    CreateTitled {
        title: String,
    },
    ChangeFilter {
        collection: CollectionView,
        tag: Option<String>,
    },
    SetState {
        id: String,
        state: NoteState,
        value: bool,
    },
    Mobile(MobilePane),
    FocusSearch,
}

#[derive(Clone, Copy)]
enum PendingCommand {
    Create,
    SetState,
}

pub struct CraneApp {
    draft: Option<DraftCoordinator>,
    worker: WorkerHandle,
    store: Store,
    store_revision: u64,
    view: ViewState,
    settings: SettingsController,
    applied_theme: Option<ThemeChoice>,
    started_at: Instant,
    next_request_id: u64,
    pending_requests: HashMap<u64, PendingCommand>,
    deferred_action: Option<DeferredAction>,
    query_cache: QueryCache,
    preview_cache: PreviewCache,
    cache_stats: CacheStats,
    request_title_focus: bool,
    request_search_focus: bool,
    mobile_pane: MobilePane,
    saved_at: Option<Instant>,
    fonts_configured: bool,
    transfers: TransferController,
    data_menu_open: bool,
    note_actions_open: bool,
    request_restore_cancel_focus: bool,
    pending_queries: HashMap<u64, PendingQuery>,
    deferred_queries: VecDeque<WorkerRequest>,
    sidebar_state: SidebarState,
    tag_tree: Vec<TagNode>,
    tag_tree_revision: Option<u64>,
    counts: SidebarCounts,
    counts_revision: Option<u64>,
    quick_open: QuickOpenState,
    request_body_focus: bool,
    completion: CompletionState,
    chooser: ChooserState,
    resolve_sequence: u64,
    resolve_link: Option<WikiLink>,
    pending_wiki_jump: Option<PendingWikiJump>,
    preview_scroll_anchor: Option<String>,
    inspector: InspectorState,
    draft_facts: Option<(String, u64, NoteFacts)>,
    committed_knowledge: Option<(String, u64, KnowledgeView)>,
    inspector_floating: bool,
    formatting: FormattingState,
    markdown_layouter: MarkdownLayouter,
}

impl CraneApp {
    pub fn new(library: Library, snapshot: LibrarySnapshot, context: egui::Context) -> Self {
        let wake_context = context.clone();
        Self::new_with_settings_and_wake(
            library,
            snapshot,
            NativeSettings::default(),
            None,
            Box::new(move || wake_context.request_repaint()),
        )
    }

    pub fn new_with_settings(
        library: Library,
        snapshot: LibrarySnapshot,
        settings: NativeSettings,
        settings_path: PathBuf,
        context: egui::Context,
    ) -> Self {
        let wake_context = context.clone();
        Self::new_with_settings_and_wake(
            library,
            snapshot,
            settings,
            Some(settings_path),
            Box::new(move || wake_context.request_repaint()),
        )
    }

    pub fn new_with_wake(
        library: Library,
        snapshot: LibrarySnapshot,
        wake: Box<dyn Fn() + Send>,
    ) -> Self {
        Self::new_with_settings_and_wake(library, snapshot, NativeSettings::default(), None, wake)
    }

    fn new_with_settings_and_wake(
        library: Library,
        snapshot: LibrarySnapshot,
        settings: NativeSettings,
        settings_path: Option<PathBuf>,
        wake: Box<dyn Fn() + Send>,
    ) -> Self {
        let selected_id = first_active_note_id(&snapshot.store);
        let draft = selected_id.as_deref().and_then(|id| {
            snapshot
                .store
                .notes
                .iter()
                .find(|note| note.id == id)
                .map(draft_from_note)
        });
        let mobile_pane = if draft.is_some() {
            MobilePane::Editor
        } else {
            MobilePane::Library
        };
        let worker = spawn_worker(library, wake);
        Self {
            draft: draft.map(DraftCoordinator::new),
            worker,
            store: snapshot.store,
            store_revision: snapshot.revision,
            view: ViewState {
                selected_id,
                ..ViewState::default()
            },
            settings: SettingsController::new(settings, settings_path),
            applied_theme: None,
            started_at: Instant::now(),
            next_request_id: GENERIC_REQUEST_START,
            pending_requests: HashMap::new(),
            deferred_action: None,
            query_cache: QueryCache::default(),
            preview_cache: PreviewCache::default(),
            cache_stats: CacheStats::default(),
            request_title_focus: false,
            request_search_focus: false,
            mobile_pane,
            saved_at: None,
            fonts_configured: false,
            transfers: TransferController::default(),
            data_menu_open: false,
            note_actions_open: false,
            request_restore_cancel_focus: false,
            pending_queries: HashMap::new(),
            deferred_queries: VecDeque::new(),
            sidebar_state: SidebarState::default(),
            tag_tree: Vec::new(),
            tag_tree_revision: None,
            counts: SidebarCounts::default(),
            counts_revision: None,
            quick_open: QuickOpenState::default(),
            request_body_focus: false,
            completion: CompletionState::default(),
            chooser: ChooserState::default(),
            resolve_sequence: 0,
            resolve_link: None,
            pending_wiki_jump: None,
            preview_scroll_anchor: None,
            inspector: InspectorState::default(),
            draft_facts: None,
            committed_knowledge: None,
            inspector_floating: false,
            formatting: FormattingState::default(),
            markdown_layouter: MarkdownLayouter::default(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.poll_worker();
        self.expire_saved_status(ui.ctx());
        if !self.fonts_configured {
            theme::install_local_fonts(ui.ctx());
            self.fonts_configured = true;
        }
        self.apply_theme(ui.ctx());
        self.handle_shortcuts(ui.ctx());

        let request_data_focus = self.transfers.take_data_focus_request();

        // Breakpoints describe the window viewport, not the central panel after
        // egui has subtracted its default inner margin.
        let mode = layout_mode(ui.ctx().viewport_rect().width());
        let focus_amount = ui.ctx().animate_bool_with_time(
            egui::Id::unique("crane-focus-mode-transition"),
            self.view.focus_mode,
            focus_transition_seconds(self.settings.value().reduced_motion),
        );
        if focus_amount > f32::EPSILON {
            self.inspector_floating = false;
        }
        if mode == LayoutMode::SinglePane && focus_amount <= f32::EPSILON {
            match self.mobile_pane {
                MobilePane::Library => {
                    theme::paint_pane(ui, theme::PaneKind::Rail, self.settings.value().theme);
                    self.show_library(ui, false, request_data_focus);
                }
                MobilePane::NoteList => {
                    theme::paint_pane(ui, theme::PaneKind::NoteList, self.settings.value().theme);
                    if ui.button("Open library").clicked() {
                        self.mobile_pane = MobilePane::Library;
                    }
                    self.show_note_list(ui);
                }
                MobilePane::Editor => {
                    theme::paint_pane(ui, theme::PaneKind::Editor, self.settings.value().theme);
                    if ui.button("Back to notes").clicked() {
                        self.request_after_save(DeferredAction::Mobile(MobilePane::NoteList));
                    }
                    self.show_editor(ui);
                }
            }
        } else if mode == LayoutMode::SinglePane {
            theme::paint_pane(ui, theme::PaneKind::Editor, self.settings.value().theme);
            self.show_editor_centered(ui);
        } else {
            let pane_height = ui.available_height();
            let chrome_factor = (1.0 - focus_amount).clamp(0.0, 1.0);
            ui.horizontal(|ui| {
                let rail_width = (if mode == LayoutMode::Wide {
                    RAIL_WIDTH
                } else {
                    COLLAPSED_RAIL_WIDTH
                }) * chrome_factor;
                if rail_width >= 1.0 {
                    ui.allocate_ui_with_layout(
                        egui::vec2(rail_width, pane_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_opacity(chrome_factor);
                            if focus_amount > f32::EPSILON {
                                ui.disable();
                            }
                            theme::paint_pane(
                                ui,
                                theme::PaneKind::Rail,
                                self.settings.value().theme,
                            );
                            self.show_library(
                                ui,
                                mode == LayoutMode::CollapsedRail,
                                request_data_focus,
                            );
                        },
                    );
                    ui.separator();
                }
                let list_width = (if mode == LayoutMode::Wide {
                    NOTE_LIST_WIDTH
                } else {
                    COLLAPSED_NOTE_LIST_WIDTH
                }) * chrome_factor;
                if list_width >= 1.0 {
                    ui.allocate_ui_with_layout(
                        egui::vec2(list_width, pane_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_opacity(chrome_factor);
                            if focus_amount > f32::EPSILON {
                                ui.disable();
                            }
                            theme::paint_pane(
                                ui,
                                theme::PaneKind::NoteList,
                                self.settings.value().theme,
                            );
                            self.show_note_list(ui);
                        },
                    );
                    ui.separator();
                }
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        theme::paint_pane(ui, theme::PaneKind::Editor, self.settings.value().theme);
                        if focus_amount > f32::EPSILON {
                            self.show_editor_centered(ui);
                        } else {
                            self.show_editor(ui);
                        }
                    },
                );
            });
        }
        self.show_restore_dialog(ui.ctx());
        self.show_quick_open(ui.ctx());
        self.show_chooser(ui.ctx());
        self.dispatch_ready_save();
        self.advance_deferred_action();
        self.dispatch_ready_transfer();
    }

    fn show_quick_open(&mut self, ctx: &egui::Context) {
        if !self.quick_open.open {
            return;
        }
        let was_open = self.quick_open.open;
        let mut outgoing = None;
        let intent = {
            let state = &mut self.quick_open;
            let mut emit = |query: Query| {
                outgoing = Some(query);
            };
            crate::quick_open::show(ctx, state, &mut emit)
        };
        if let Some(query) = outgoing {
            let sequence = self.quick_open.sequence;
            self.send_query(query, PendingQuery::QuickOpen(sequence));
        }
        if let Some(QuickOpenIntent::OpenNote(note_id)) = intent {
            self.request_after_save(DeferredAction::Select(note_id));
        }
        if was_open && !self.quick_open.open {
            self.request_body_focus = true;
        }
    }

    fn show_chooser(&mut self, ctx: &egui::Context) {
        if !self.chooser.open {
            return;
        }
        let was_open = self.chooser.open;
        let intent = crate::knowledge_navigation::show_chooser(ctx, &mut self.chooser);
        match intent {
            Some(ChooserIntent::OpenNote(note_id)) => {
                self.apply_nav_intent(KnowledgeNavIntent::OpenNote {
                    note_id,
                    heading: None,
                });
            }
            Some(ChooserIntent::CreateNote(title)) => {
                self.apply_nav_intent(KnowledgeNavIntent::CreateNote { title });
            }
            Some(ChooserIntent::Close) | None => {}
        }
        if was_open && !self.chooser.open {
            self.request_body_focus = true;
        }
    }

    fn apply_nav_intent(&mut self, intent: KnowledgeNavIntent) {
        match intent {
            KnowledgeNavIntent::OpenNote { note_id, heading } => {
                self.request_body_focus = true;
                self.request_after_save(DeferredAction::Navigate { note_id, heading });
            }
            KnowledgeNavIntent::CreateNote { title } => {
                self.request_after_save(DeferredAction::CreateTitled { title });
            }
        }
    }

    fn apply_resolution(&mut self, view: ResolutionView) {
        match view {
            ResolutionView::Resolved { note_id, heading } => {
                self.apply_nav_intent(KnowledgeNavIntent::OpenNote {
                    note_id,
                    heading: Some(heading),
                });
            }
            ResolutionView::Unresolved { target_title } => {
                crate::knowledge_navigation::open_chooser(
                    &mut self.chooser,
                    target_title,
                    Vec::new(),
                );
            }
            ResolutionView::Ambiguous { candidates } => {
                let title = candidates
                    .first()
                    .map(|candidate| candidate.title.clone())
                    .or_else(|| {
                        self.resolve_link
                            .as_ref()
                            .map(|link| link.target_title.clone())
                    })
                    .unwrap_or_default();
                crate::knowledge_navigation::open_chooser(&mut self.chooser, title, candidates);
            }
        }
    }

    pub fn visible_note_ids(&mut self) -> &[String] {
        self.ensure_note_ids();
        &self.query_cache.ids
    }

    fn send_query(&mut self, query: Query, pending: PendingQuery) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        enqueue_deferred_query(
            &mut self.deferred_queries,
            &mut self.pending_queries,
            WorkerRequest::query(request_id, query),
            pending,
        );
        self.dispatch_ready_queries();
        request_id
    }

    fn dispatch_ready_queries(&mut self) {
        let failures = flush_deferred_queries(&mut self.deferred_queries, &mut |request| {
            self.worker.try_send(request)
        });
        for (request_id, error) in failures {
            self.pending_queries.remove(&request_id);
            self.view.status = Some(error.message);
        }
    }

    fn ensure_sidebar_queries(&mut self) {
        let has_tag = self
            .pending_queries
            .values()
            .any(|q| matches!(q, PendingQuery::TagTree));
        if self.tag_tree_revision != Some(self.store_revision) && !has_tag {
            self.send_query(
                Query::TagTree {
                    include_archived: true,
                },
                PendingQuery::TagTree,
            );
        }
        let has_counts = self
            .pending_queries
            .values()
            .any(|q| matches!(q, PendingQuery::Counts));
        if self.counts_revision != Some(self.store_revision) && !has_counts {
            self.send_query(Query::CollectionCounts, PendingQuery::Counts);
        }
    }

    fn ensure_note_ids(&mut self) {
        let key: ListKey = (
            self.store_revision,
            self.view.collection,
            self.view.tag.clone(),
            self.view.query.clone(),
        );
        if self.query_cache.key.as_ref() == Some(&key) {
            return;
        }
        let already = self
            .pending_queries
            .values()
            .any(|q| matches!(q, PendingQuery::NoteIds(k) if *k == key));
        if already {
            return;
        }
        self.send_query(
            Query::NotesForCollection {
                collection: key.1,
                tag_path: key.2.clone(),
                query: key.3.clone(),
            },
            PendingQuery::NoteIds(key),
        );
    }

    fn handle_query_response(
        &mut self,
        request_id: u64,
        response: crane_application::knowledge::QueryResponse,
    ) {
        let Some(pending) = self.pending_queries.remove(&request_id) else {
            return;
        };
        match (pending, response.data) {
            (PendingQuery::NoteIds(key), QueryData::NoteIds(ids)) => {
                let current: ListKey = (
                    self.store_revision,
                    self.view.collection,
                    self.view.tag.clone(),
                    self.view.query.clone(),
                );
                if key == current {
                    self.query_cache.ids = ids;
                    self.query_cache.key = Some(key);
                    self.cache_stats.query_recomputes =
                        self.cache_stats.query_recomputes.saturating_add(1);
                    let selected_visible = self
                        .view
                        .selected_id
                        .as_ref()
                        .is_some_and(|id| self.query_cache.ids.contains(id));
                    let draft_busy = self
                        .draft
                        .as_ref()
                        .is_some_and(DraftCoordinator::has_pending_work);
                    if !selected_visible
                        && self.view.selected_id.is_some()
                        && !draft_busy
                        && self.pending_requests.is_empty()
                    {
                        self.view.selected_id = None;
                        self.draft = None;
                        self.preview_cache.key = None;
                    }
                }
            }
            (PendingQuery::TagTree, QueryData::TagTree(tree)) => {
                self.tag_tree = tree;
                self.tag_tree_revision = Some(response.store_revision);
            }
            (PendingQuery::Counts, QueryData::CollectionCounts(counts)) => {
                self.counts = counts;
                self.counts_revision = Some(response.store_revision);
            }
            (PendingQuery::QuickOpen(sequence), QueryData::QuickOpen(hits))
                if sequence == self.quick_open.sequence =>
            {
                self.quick_open.active = self.quick_open.active.min(hits.len().saturating_sub(1));
                self.quick_open.hits = hits;
            }
            (PendingQuery::CompleteWikiLink(sequence), QueryData::Completion(candidates))
                if sequence == self.completion.sequence =>
            {
                self.completion.active = self
                    .completion
                    .active
                    .min(candidates.len().saturating_sub(1));
                self.completion.candidates = candidates;
            }
            (PendingQuery::ResolveWikiLink(sequence), QueryData::Resolution(view))
                if sequence == self.resolve_sequence =>
            {
                self.apply_resolution(view);
            }
            (PendingQuery::KnowledgeFor { note_id, revision }, QueryData::Knowledge(view)) => {
                self.committed_knowledge = Some((note_id, revision, *view));
            }
            _ => {}
        }
    }

    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            markdown_parses: self.markdown_layouter.parse_count(),
            ..self.cache_stats
        }
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.view.query = query.into();
    }

    pub fn selected_note_id(&self) -> Option<&str> {
        self.view.selected_id.as_deref()
    }

    pub fn selected_draft(&self) -> Option<&Draft> {
        self.draft.as_ref().map(DraftCoordinator::current)
    }

    pub fn status(&self) -> Option<&str> {
        self.view.status.as_deref()
    }

    pub fn save_now(&mut self) {
        if let Some(draft) = &mut self.draft {
            draft.save_now();
        }
        self.dispatch_ready_save();
    }

    pub fn has_pending_work(&self) -> bool {
        !self.pending_requests.is_empty()
            || self.deferred_action.is_some()
            || self.transfers.has_pending_work()
            || !self.pending_queries.is_empty()
            || self
                .draft
                .as_ref()
                .is_some_and(DraftCoordinator::has_pending_work)
    }

    pub fn edit_selected_for_test(&mut self, title: &str, body: &str) {
        let now = self.now_millis();
        if let Some(draft) = &mut self.draft {
            draft.edit(title.into(), body.into(), now);
        }
    }

    pub fn simulate_save_failure_for_test(&mut self, message: &str) {
        let now = self.now_millis();
        let Some(draft) = &mut self.draft else {
            return;
        };
        draft.save_now();
        if let Some(request) = draft.take_ready_request(now) {
            draft.complete_failure(
                request.request_id,
                AppError::new(ErrorCode::Io, message.to_owned()),
            );
            self.view.status = Some(message.to_owned());
        }
    }

    fn now_millis(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    fn show_library(&mut self, ui: &mut egui::Ui, collapsed: bool, request_data_focus: bool) {
        self.ensure_sidebar_queries();
        let transfer_status: Option<String> = match self.transfers.state() {
            TransferState::Flushing => self
                .transfers
                .status()
                .map(str::to_owned)
                .or_else(|| Some("Saving before transfer".to_string())),
            TransferState::Running => Some("Transfer in progress".to_string()),
            _ => self.transfers.status().map(str::to_owned),
        };
        // Reserve enough room for the separator, two 44-point actions, spacing,
        // and the reduced-motion checkbox. A 128-point footer left the final
        // preference below the viewport at the supported 560-point minimum.
        let footer_height = if ui.available_height() <= 600.0 {
            176.0
        } else {
            160.0
        };
        let mutations_enabled = self.transfers.can_mutate();
        let content_size = egui::vec2(
            ui.available_width(),
            (ui.available_height() - footer_height).max(0.0),
        );
        let mut intent = None;
        ui.allocate_ui_with_layout(
            content_size,
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                intent = crate::sidebar::show(
                    ui,
                    &mut self.sidebar_state,
                    SidebarView {
                        counts: &self.counts,
                        tag_tree: &self.tag_tree,
                        active: self.view.collection,
                        active_tag: self.view.tag.as_deref(),
                        collapsed,
                        mutations_enabled,
                        theme: self.settings.value().theme,
                    },
                );
            },
        );
        match intent {
            Some(SidebarIntent::Create) => {
                self.request_after_save(DeferredAction::Create);
            }
            Some(SidebarIntent::Collection(collection)) => {
                self.request_after_save(DeferredAction::ChangeFilter {
                    collection,
                    tag: None,
                });
            }
            Some(SidebarIntent::Tag(tag)) => {
                self.request_after_save(DeferredAction::ChangeFilter {
                    collection: CollectionView::Notes,
                    tag,
                });
            }
            None => {}
        }
        ui.separator();
        let actions = library::show(
            ui,
            library::LibraryView {
                theme: self.settings.value().theme,
                reduced_motion: self.settings.value().reduced_motion,
                collapsed,
                mutations_enabled,
                data_menu_open: &mut self.data_menu_open,
                request_data_focus,
                transfer_status: transfer_status.as_deref(),
            },
        );
        if actions.toggle_theme {
            let mut candidate = self.settings.value().clone();
            candidate.theme = match candidate.theme {
                ThemeChoice::Dark => ThemeChoice::Light,
                ThemeChoice::Light => ThemeChoice::Dark,
            };
            self.persist_settings(candidate);
        }
        if let Some(value) = actions.reduced_motion {
            let mut candidate = self.settings.value().clone();
            candidate.reduced_motion = value;
            self.persist_settings(candidate);
        }
        if let Some(action) = actions.transfer {
            let command = match action {
                library::DataAction::ImportMarkdown => Command::ImportMarkdown,
                library::DataAction::ExportLibrary => Command::ExportLibrary,
                library::DataAction::CreateBackup => Command::CreateBackup,
                library::DataAction::InspectBackup => Command::InspectBackup,
            };
            self.begin_transfer(command);
        }
    }

    fn show_note_list(&mut self, ui: &mut egui::Ui) {
        self.ensure_note_ids();
        let visible = self.visible_note_ids().to_vec();
        let previous_query = self.view.query.clone();
        let list_action = note_list::show(
            ui,
            &self.store,
            &visible,
            self.view.selected_id.as_deref(),
            &mut self.view.query,
            std::mem::take(&mut self.request_search_focus),
            collection_label(self.view.collection),
            self.settings.value().theme,
            self.transfers.can_mutate(),
        );
        if self.view.query != previous_query {
            self.query_cache.key = None;
        }
        if list_action.quick_open {
            crate::quick_open::open(&mut self.quick_open, self.view.selected_id.clone());
        }
        if list_action.clear_search {
            self.view.query.clear();
            self.query_cache.key = None;
            self.request_search_focus = true;
        }
        if list_action.create && self.transfers.can_mutate() {
            self.request_after_save(DeferredAction::Create);
        }
        if let Some(id) = list_action.selected {
            self.request_after_save(DeferredAction::Select(id));
        }
    }

    fn show_editor(&mut self, ui: &mut egui::Ui) {
        let has_selection = self.view.selected_id.is_some();
        let docked = has_selection
            && self.inspector.open
            && inspector_presentation(ui.available_width()) == InspectorPresentation::Docked;
        self.inspector_floating = has_selection && self.inspector.open && !docked;
        let inspector_was_open = self.inspector.open;
        if docked {
            ui.horizontal(|ui| {
                let main_width = (ui.available_width() - INSPECTOR_WIDTH).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(main_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.show_editor_centered(ui),
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(INSPECTOR_WIDTH, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.show_inspector(ui),
                );
            });
        } else {
            self.show_editor_centered(ui);
            if self.inspector_floating {
                egui::Window::new("Inspector")
                    .id(egui::Id::unique("inspector-float"))
                    .frame(theme::overlay_frame(self.settings.value().theme))
                    .anchor(egui::Align2::RIGHT_TOP, [-16.0, 16.0])
                    .default_size([INSPECTOR_WIDTH, 480.0])
                    .show(ui.ctx(), |ui| self.show_inspector(ui));
            }
        }
        if inspector_was_open && !self.inspector.open {
            self.request_body_focus = true;
        }
    }

    fn show_editor_centered(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let measure = editor_measure(available.x);
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(measure, available.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.show_editor_main(ui),
                );
            },
        );
    }

    fn show_editor_main(&mut self, ui: &mut egui::Ui) {
        let Some(selected_id) = self.view.selected_id.clone() else {
            ui.heading("A quiet place for bright ideas");
            if ui
                .add_enabled(
                    self.pending_requests.is_empty() && self.transfers.can_mutate(),
                    egui::Button::new("New note"),
                )
                .clicked()
            {
                self.request_after_save(DeferredAction::Create);
            }
            return;
        };
        let Some(note) = self
            .store
            .notes
            .iter()
            .find(|note| note.id == selected_id)
            .cloned()
        else {
            ui.label("This note is no longer available.");
            return;
        };
        let preview_document = if self.view.preview {
            self.preview_document().cloned()
        } else {
            None
        };
        let request_focus = std::mem::take(&mut self.request_title_focus);
        let request_body_focus = std::mem::take(&mut self.request_body_focus);
        let status = self.view.status.clone();
        let now = self.now_millis();
        let mutation_enabled = self.pending_requests.is_empty() && self.transfers.can_mutate();
        let scroll_anchor = if self.view.preview {
            self.preview_scroll_anchor.take()
        } else {
            None
        };
        let Some(draft) = &mut self.draft else {
            return;
        };
        let mut caret_jump = None;
        if self
            .pending_wiki_jump
            .as_ref()
            .is_some_and(|jump| jump.note_id == draft.current().note_id)
        {
            let jump = self
                .pending_wiki_jump
                .take()
                .expect("wiki jump checked above");
            if self.view.preview {
                self.preview_scroll_anchor = Some(jump.anchor);
            } else {
                caret_jump = Some(crate::knowledge_navigation::byte_to_char(
                    &draft.current().body,
                    jump.byte_offset,
                ));
            }
        }
        let flags = editor::NoteFlags {
            pinned: note.pinned,
            archived: note.archived,
            trashed: note.trashed,
        };
        let mut completion_outgoing = None;
        let actions = if mutation_enabled {
            let mut emit = |query: Query| {
                completion_outgoing = Some(query);
            };
            editor::show(
                ui,
                draft,
                editor::EditorView {
                    flags,
                    preview: self.view.preview,
                    preview_document: preview_document.as_ref(),
                    preview_scroll_anchor: scroll_anchor.as_deref(),
                    request_title_focus: request_focus,
                    request_body_focus,
                    caret_jump,
                    status: status.as_deref(),
                    mutations_enabled: true,
                    note_actions_open: &mut self.note_actions_open,
                    completion: Some(editor::CompletionView {
                        state: &mut self.completion,
                        emit_query: &mut emit,
                    }),
                    formatting: Some(editor::FormattingView {
                        state: &mut self.formatting,
                    }),
                    theme: self.settings.value().theme,
                    markdown_layouter: &mut self.markdown_layouter,
                },
            )
        } else {
            let mut display_draft = draft.clone();
            editor::show(
                ui,
                &mut display_draft,
                editor::EditorView {
                    flags,
                    preview: self.view.preview,
                    preview_document: preview_document.as_ref(),
                    preview_scroll_anchor: scroll_anchor.as_deref(),
                    request_title_focus: false,
                    request_body_focus: false,
                    caret_jump: None,
                    status: status.as_deref(),
                    mutations_enabled: false,
                    note_actions_open: &mut self.note_actions_open,
                    completion: None,
                    formatting: None,
                    theme: self.settings.value().theme,
                    markdown_layouter: &mut self.markdown_layouter,
                },
            )
        };
        if mutation_enabled && actions.changed {
            draft.mark_edited(now);
            self.preview_cache.key = None;
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(
                    crate::draft::SAVE_DELAY_MILLIS,
                ));
        }
        if mutation_enabled && actions.save_now {
            draft.save_now();
        }
        if actions.retry {
            draft.retry();
        }
        if mutation_enabled && actions.toggle_preview {
            self.view.preview = !self.view.preview;
        }
        if actions.toggle_inspector {
            self.inspector.open = !self.inspector.open;
        }
        if actions.toggle_focus_mode {
            let exiting = self.view.focus_mode;
            self.view.focus_mode = !self.view.focus_mode;
            if exiting {
                self.request_body_focus = true;
            }
        }
        if mutation_enabled && actions.export_note {
            self.begin_transfer(Command::ExportNote {
                id: selected_id.clone(),
            });
        }
        if let Some(error) = actions.link_error {
            self.view.status = Some(error);
        }
        if mutation_enabled && actions.toggle_pin {
            self.request_after_save(DeferredAction::SetState {
                id: selected_id.clone(),
                state: NoteState::Pinned,
                value: !note.pinned,
            });
        }
        if mutation_enabled && actions.toggle_archive {
            self.request_after_save(DeferredAction::SetState {
                id: selected_id.clone(),
                state: NoteState::Archived,
                value: !note.archived,
            });
        }
        if mutation_enabled && actions.toggle_trash {
            self.request_after_save(DeferredAction::SetState {
                id: selected_id,
                state: NoteState::Trashed,
                value: !note.trashed,
            });
        }
        if let Some(query) = completion_outgoing {
            let sequence = self.completion.sequence;
            self.send_query(query, PendingQuery::CompleteWikiLink(sequence));
        }
        if let Some(link) = actions.wiki_link {
            self.resolve_link = Some(link.clone());
            self.resolve_sequence = self.resolve_sequence.saturating_add(1);
            let sequence = self.resolve_sequence;
            self.send_query(
                Query::ResolveWikiLink { link },
                PendingQuery::ResolveWikiLink(sequence),
            );
        }
    }

    fn show_inspector(&mut self, ui: &mut egui::Ui) {
        let Some(draft) = self.draft.as_ref() else {
            return;
        };
        let (note_id, revision, dirty) = {
            let current = draft.current();
            (
                current.note_id.clone(),
                current.revision,
                current.is_dirty(),
            )
        };
        let facts_fresh = self
            .draft_facts
            .as_ref()
            .is_some_and(|(id, rev, _)| *id == note_id && *rev == revision);
        if !facts_fresh {
            let current = self.draft.as_ref().expect("draft checked above").current();
            let facts = parse_draft_facts(&current.note_id, &current.title, &current.body);
            self.draft_facts = Some((note_id.clone(), revision, facts));
        }
        let store_revision = self.store_revision;
        let knowledge_fresh = self
            .committed_knowledge
            .as_ref()
            .is_some_and(|(id, rev, _)| *id == note_id && *rev == store_revision);
        let knowledge_pending = self.pending_queries.values().any(
            |query| matches!(query, PendingQuery::KnowledgeFor { note_id: id, revision: rev } if id == &note_id && *rev == store_revision),
        );
        if !knowledge_fresh && !knowledge_pending {
            self.send_query(
                Query::KnowledgeFor {
                    note_id: note_id.clone(),
                },
                PendingQuery::KnowledgeFor {
                    note_id: note_id.clone(),
                    revision: store_revision,
                },
            );
        }
        let facts = self
            .draft_facts
            .as_ref()
            .map(|(_, _, facts)| facts)
            .expect("draft facts cached above");
        let committed = self
            .committed_knowledge
            .as_ref()
            .and_then(|(id, rev, view)| (*id == note_id && *rev == store_revision).then_some(view));
        let theme = self.settings.value().theme;
        let intent =
            crate::inspector::show(ui, &mut self.inspector, facts, committed, dirty, theme);
        match intent {
            InspectorIntent::OpenNote { note_id, heading } => {
                self.apply_nav_intent(KnowledgeNavIntent::OpenNote { note_id, heading });
            }
            InspectorIntent::JumpToHeading { range } => {
                if self.view.preview {
                    let anchor = self
                        .committed_knowledge
                        .as_ref()
                        .and_then(|(id, rev, view)| {
                            if *id == note_id && *rev == store_revision {
                                view.outline
                                    .iter()
                                    .find(|item| item.range == range)
                                    .map(|item| item.anchor.clone())
                            } else {
                                None
                            }
                        });
                    if let Some(anchor) = anchor {
                        self.preview_scroll_anchor = Some(anchor);
                    }
                } else {
                    self.pending_wiki_jump = Some(PendingWikiJump {
                        note_id: note_id.clone(),
                        byte_offset: range.start,
                        anchor: String::new(),
                    });
                }
            }
            InspectorIntent::None => {}
        }
    }

    fn preview_document(&mut self) -> Option<&MarkdownDocument> {
        let draft = self.draft.as_ref()?.current();
        let key = (draft.note_id.clone(), draft.revision);
        if self.preview_cache.key.as_ref() != Some(&key) {
            self.preview_cache.document = Some(parse_preview(&draft.body));
            self.preview_cache.key = Some(key);
            self.cache_stats.preview_recomputes =
                self.cache_stats.preview_recomputes.saturating_add(1);
        }
        self.preview_cache.document.as_ref()
    }

    fn select_note(&mut self, id: &str) {
        if self.view.selected_id.as_deref() == Some(id) {
            self.mobile_pane = MobilePane::Editor;
            return;
        }
        let Some(note) = self.store.notes.iter().find(|note| note.id == id) else {
            return;
        };
        let next = draft_from_note(note);
        let switched = match &mut self.draft {
            Some(draft) => draft.request_switch(next),
            None => {
                self.draft = Some(DraftCoordinator::new(next));
                true
            }
        };
        if switched {
            self.view.selected_id = Some(id.to_owned());
            self.preview_cache.key = None;
            self.mobile_pane = MobilePane::Editor;
        } else {
            self.view.status = Some("Saving before switching notes…".into());
        }
    }

    fn request_after_save(&mut self, action: DeferredAction) {
        if self.deferred_action.is_some() {
            self.view.status = Some("Finish the pending action before starting another.".into());
            return;
        }
        let must_wait = self
            .draft
            .as_ref()
            .is_some_and(DraftCoordinator::has_pending_work);
        if !must_wait {
            self.execute_deferred_action(action);
            return;
        }

        self.deferred_action = Some(action);
        if let Some(draft) = &mut self.draft {
            if draft.last_error().is_none() {
                draft.save_now();
                self.view.status = Some("Saving before continuing…".into());
            }
        }
    }

    fn advance_deferred_action(&mut self) {
        let ready = self
            .draft
            .as_ref()
            .is_none_or(|draft| !draft.has_pending_work() && draft.last_error().is_none());
        if ready {
            if let Some(action) = self.deferred_action.take() {
                self.execute_deferred_action(action);
            }
        }
    }

    fn execute_deferred_action(&mut self, action: DeferredAction) {
        match action {
            DeferredAction::Create => {
                self.dispatch(Command::CreateNote, PendingCommand::Create);
                self.view.status = Some("Creating note…".into());
            }
            DeferredAction::Select(id) => self.select_note(&id),
            DeferredAction::Navigate { note_id, heading } => {
                self.select_note(&note_id);
                match heading {
                    Some(HeadingResolution::Found { anchor, range }) => {
                        self.pending_wiki_jump = Some(PendingWikiJump {
                            note_id,
                            byte_offset: range.start,
                            anchor,
                        });
                    }
                    Some(HeadingResolution::Missing) => {
                        self.view.status = Some("Heading not found".into());
                    }
                    Some(HeadingResolution::NotRequested) | None => {}
                }
            }
            DeferredAction::CreateTitled { title } => {
                self.dispatch(
                    Command::CreateNoteWithTitle { title },
                    PendingCommand::Create,
                );
                self.view.status = Some("Creating note…".into());
            }
            DeferredAction::ChangeFilter { collection, tag } => {
                self.view.collection = collection;
                self.view.tag = tag;
                self.view.selected_id = None;
                self.draft = None;
                self.preview_cache.key = None;
                self.mobile_pane = MobilePane::NoteList;
            }
            DeferredAction::SetState { id, state, value } => {
                self.dispatch(
                    Command::SetNoteState { id, state, value },
                    PendingCommand::SetState,
                );
                self.view.status = Some("Updating note…".into());
            }
            DeferredAction::Mobile(pane) => {
                self.close_floating_inspector();
                self.mobile_pane = pane;
            }
            DeferredAction::FocusSearch => {
                self.mobile_pane = MobilePane::NoteList;
                self.request_search_focus = true;
            }
        }
    }

    fn dispatch_ready_save(&mut self) {
        let now = self.now_millis();
        let Some(request) = self
            .draft
            .as_mut()
            .and_then(|draft| draft.take_ready_request(now))
        else {
            return;
        };
        if let Err(error) = self.worker.try_send(request.clone()) {
            if let Some(draft) = &mut self.draft {
                if error.code == ErrorCode::QueueFull {
                    draft.return_unsent(request);
                } else {
                    draft.complete_failure(request.request_id, error.clone());
                    self.transfers.abort_prerequisite_save();
                }
            }
            self.view.status = Some(error.message);
        } else {
            self.view.status = Some("Saving…".into());
        }
    }

    fn begin_transfer(&mut self, command: Command) {
        let draft = self.selected_draft().cloned();
        let result = match draft.as_ref() {
            Some(draft) => self.transfers.begin(command, draft),
            None => self.transfers.begin_without_draft(command),
        };
        if let Err(error) = result {
            self.view.status = Some(error.message);
            return;
        }
        if matches!(self.transfers.state(), TransferState::Flushing) {
            if let Some(draft) = &mut self.draft {
                if draft.last_error().is_some() {
                    self.transfers.abort_prerequisite_save();
                    return;
                }
                draft.save_now();
            }
        }
        self.dispatch_ready_save();
        self.dispatch_ready_transfer();
    }

    fn dispatch_ready_transfer(&mut self) {
        let current = self.selected_draft().cloned();
        let Some(request) = self.transfers.take_ready_request(current.as_ref()) else {
            return;
        };
        if let Err(error) = self.worker.try_send(request.clone()) {
            let terminal = error.code != ErrorCode::QueueFull;
            self.transfers.return_unsent(request, error);
            self.view.status = self.transfers.status().map(str::to_owned);
            if terminal {
                self.mobile_pane = MobilePane::Library;
            }
        } else {
            self.view.status = Some("Transfer in progress…".into());
        }
    }

    fn show_restore_dialog(&mut self, context: &egui::Context) {
        let state = self.transfers.state().clone();
        let action = self.transfers.restore_dialog().map(|model| {
            restore::show(
                context,
                model,
                std::mem::take(&mut self.request_restore_cancel_focus),
            )
        });
        match (action, state) {
            (
                Some(restore::RestoreAction::Confirm),
                TransferState::AwaitingConfirmation { token, .. },
            ) => self.begin_transfer(Command::RestoreBackup { token }),
            (
                Some(restore::RestoreAction::Cancel),
                TransferState::AwaitingConfirmation { token, .. },
            ) => self.begin_transfer(Command::CancelRestore { token }),
            _ => {}
        }
    }

    fn dispatch(&mut self, command: Command, pending: PendingCommand) {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        match self
            .worker
            .try_send(WorkerRequest::command(request_id, command))
        {
            Ok(()) => {
                self.pending_requests.insert(request_id, pending);
            }
            Err(error) => self.view.status = Some(error.message),
        }
    }

    fn poll_worker(&mut self) {
        while let Some(result) = self.worker.try_recv() {
            if self.pending_queries.contains_key(&result.request_id) {
                if let WorkerOutcome::Query(response) = result.outcome {
                    self.handle_query_response(result.request_id, response);
                }
                continue;
            }
            if self.transfers.owns_request(result.request_id) {
                let was_inspection = matches!(self.transfers.state(), TransferState::Running);
                let effect = self.transfers.complete(result);
                if matches!(
                    self.transfers.state(),
                    TransferState::AwaitingConfirmation { .. }
                ) {
                    self.request_restore_cancel_focus = was_inspection;
                } else {
                    self.mobile_pane = MobilePane::Library;
                }
                if let TransferEffect::ReplaceStore(store) = effect {
                    self.replace_store(store);
                }
                self.view.status = self.transfers.status().map(str::to_owned);
            } else {
                self.accept_worker_result(result);
            }
        }
        self.dispatch_ready_queries();
        if let Some(draft) = &self.draft {
            let current_id = draft.current().note_id.clone();
            if self.view.selected_id.as_deref() != Some(current_id.as_str())
                && !draft.has_pending_work()
            {
                self.view.selected_id = Some(current_id);
                self.preview_cache.key = None;
            }
        }
    }

    fn replace_store(&mut self, store: Store) {
        self.store = store;
        self.store_revision = self.store_revision.saturating_add(1);
        self.query_cache.key = None;
        self.preview_cache.key = None;
        let selected = self
            .view
            .selected_id
            .as_deref()
            .and_then(|id| self.store.notes.iter().find(|note| note.id == id))
            .or_else(|| {
                first_active_note_id(&self.store)
                    .and_then(|id| self.store.notes.iter().find(|note| note.id == id))
            });
        self.view.selected_id = selected.map(|note| note.id.clone());
        self.draft = selected.map(|note| DraftCoordinator::new(draft_from_note(note)));
    }

    fn accept_worker_result(&mut self, result: WorkerResult) {
        let pending = self.pending_requests.remove(&result.request_id);
        if pending.is_none() {
            if let WorkerOutcome::Command(Ok(command_result)) = &result.outcome {
                if let CommandData::Saved { note, .. } = &command_result.data {
                    self.replace_note(note.clone(), command_result.revision);
                }
            }
            if let Some(draft) = &mut self.draft {
                draft.complete(result);
                if let Some(error) = draft.last_error() {
                    self.transfers.abort_prerequisite_save();
                    self.view.status = Some(error.message.clone());
                    self.saved_at = None;
                } else {
                    self.mark_saved();
                }
            }
            return;
        }

        let pending = pending.expect("generic result was identified above");
        let WorkerOutcome::Command(result) = result.outcome else {
            return;
        };
        match result {
            Ok(command_result) => {
                self.store_revision = command_result.revision;
                match command_result.data {
                    CommandData::Created(note) => {
                        self.store.notes.push(note.clone());
                        self.draft = Some(DraftCoordinator::new(draft_from_note(&note)));
                        self.view.selected_id = Some(note.id);
                        self.view.collection = CollectionView::Notes;
                        self.view.tag = None;
                        self.view.query.clear();
                        self.request_title_focus = true;
                        self.mobile_pane = MobilePane::Editor;
                    }
                    CommandData::StateChanged(note) => {
                        self.replace_note(note, command_result.revision);
                        if matches!(pending, PendingCommand::SetState) {
                            self.reconcile_selected_visibility();
                        }
                    }
                    CommandData::Imported { store, .. } | CommandData::Restored(store) => {
                        self.store = store;
                    }
                    _ => {}
                }
                self.query_cache.key = None;
                self.mark_saved();
            }
            Err(error) => self.view.status = Some(error.message),
        }
    }

    fn replace_note(&mut self, note: Note, revision: u64) {
        if let Some(existing) = self.store.notes.iter_mut().find(|item| item.id == note.id) {
            *existing = note;
        }
        self.store_revision = revision;
        self.query_cache.key = None;
    }

    fn reconcile_selected_visibility(&mut self) {
        let visible = self.query_cache.ids.clone();
        if self
            .view
            .selected_id
            .as_ref()
            .is_some_and(|selected| !visible.contains(selected))
        {
            self.view.selected_id = None;
            self.draft = None;
            self.preview_cache.key = None;
            self.mobile_pane = MobilePane::NoteList;
        }
    }

    fn mark_saved(&mut self) {
        self.view.status = Some("Saved".into());
        self.saved_at = Some(Instant::now());
    }

    fn expire_saved_status(&mut self, context: &egui::Context) {
        let Some(saved_at) = self.saved_at else {
            return;
        };
        let duration = if self.settings.value().reduced_motion {
            std::time::Duration::from_millis(200)
        } else {
            std::time::Duration::from_millis(1_200)
        };
        let elapsed = saved_at.elapsed();
        if elapsed >= duration {
            if self.view.status.as_deref() == Some("Saved") {
                self.view.status = None;
            }
            self.saved_at = None;
        } else {
            context.request_repaint_after(duration - elapsed);
        }
    }

    fn apply_theme(&mut self, context: &egui::Context) {
        let choice = self.settings.value().theme;
        if self.applied_theme != Some(choice) {
            theme::apply(context, choice);
            self.applied_theme = Some(choice);
        }
    }

    fn persist_settings(&mut self, candidate: NativeSettings) {
        match self.settings.commit(candidate) {
            Ok(()) => self.applied_theme = None,
            Err(error) => self.view.status = Some(error),
        }
    }

    fn handle_shortcuts(&mut self, context: &egui::Context) {
        if self.transfers.restore_dialog().is_some() {
            return;
        }
        let (new_note, save, preview, search, escape, quick_open) = context.input(|input| {
            let primary_modifier = input.modifiers.command || input.modifiers.ctrl;
            (
                primary_modifier && input.key_pressed(egui::Key::N),
                primary_modifier && input.key_pressed(egui::Key::S),
                primary_modifier && input.key_pressed(egui::Key::P),
                primary_modifier && input.key_pressed(egui::Key::F),
                input.key_pressed(egui::Key::Escape),
                primary_modifier && input.key_pressed(egui::Key::O),
            )
        });
        if self.transfers.restore_dialog().is_none() && quick_open && self.transfers.can_mutate() {
            if self.quick_open.open {
                crate::quick_open::close(&mut self.quick_open);
            } else {
                crate::quick_open::open(&mut self.quick_open, self.view.selected_id.clone());
            }
        }
        if new_note && self.transfers.can_mutate() {
            self.request_after_save(DeferredAction::Create);
        }
        if save && self.transfers.can_mutate() {
            self.save_now();
        }
        if preview && self.transfers.can_mutate() {
            self.view.preview = !self.view.preview;
        }
        if search && self.transfers.can_mutate() {
            self.request_after_save(DeferredAction::FocusSearch);
        }
        if escape && self.transfers.can_mutate() {
            self.dismiss_top_for_escape();
        }
    }

    fn close_floating_inspector(&mut self) {
        if self.inspector_floating {
            self.inspector.open = false;
        }
    }

    /// Central Escape stack: completion, Quick Open, and the chooser consume
    /// their own Escape during rendering, so reaching here means dismissing
    /// the floating inspector, then focus mode, then single-pane back.
    fn dismiss_top_for_escape(&mut self) {
        if self.completion.popup_visible() || self.quick_open.open || self.chooser.open {
            return;
        }
        if self.note_actions_open {
            self.note_actions_open = false;
            self.request_body_focus = true;
            return;
        }
        if self.inspector.open && self.inspector_floating {
            self.inspector.open = false;
            self.request_body_focus = true;
            return;
        }
        if self.view.focus_mode {
            self.view.focus_mode = false;
            self.request_body_focus = true;
            return;
        }
        let target = match self.mobile_pane {
            MobilePane::Editor => MobilePane::NoteList,
            MobilePane::NoteList => MobilePane::Library,
            MobilePane::Library => MobilePane::Library,
        };
        self.request_after_save(DeferredAction::Mobile(target));
    }

    fn logic(&mut self, context: &egui::Context) {
        let close_requested = context.input(|input| input.viewport().close_requested());
        if close_requested
            && self
                .draft
                .as_mut()
                .is_some_and(|draft| !draft.request_close())
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.poll_worker();
        self.dispatch_ready_save();
        self.advance_deferred_action();
        self.dispatch_ready_transfer();
        if self.draft.as_ref().is_some_and(DraftCoordinator::can_close) {
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if let Some(delay) = self
            .draft
            .as_ref()
            .and_then(|draft| draft.next_save_delay_millis(self.now_millis()))
        {
            context.request_repaint_after(std::time::Duration::from_millis(delay));
        }
    }
}

impl eframe::App for CraneApp {
    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.logic(context);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| self.show(ui));
    }
}

fn collection_label(collection: CollectionView) -> &'static str {
    match collection {
        CollectionView::Notes => "Notes",
        CollectionView::Pinned => "Pinned",
        CollectionView::Today => "Today",
        CollectionView::Todos => "Todos",
        CollectionView::Untagged => "Untagged",
        CollectionView::Archive => "Archive",
        CollectionView::Trash => "Trash",
    }
}

fn first_active_note_id(store: &Store) -> Option<String> {
    let mut notes: Vec<&Note> = store
        .notes
        .iter()
        .filter(|note| !note.archived && !note.trashed)
        .collect();
    notes.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    notes.first().map(|note| note.id.clone())
}

fn draft_from_note(note: &Note) -> Draft {
    Draft {
        note_id: note.id.clone(),
        title: note.title.clone(),
        body: note.body.clone(),
        revision: 0,
        acknowledged_revision: 0,
    }
}
