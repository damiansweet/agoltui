use crate::data::{ItemIssue, LoadEvent, LoadedData, owner_counts, spawn_loader};
use agol::{ArcGISReferences, ArcGISSearchResults};
use eframe::egui::{self, Color32, RichText};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Content,
    Connections,
    Owners,
    BrokenConnections,
    StructureIssues,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchField {
    Title,
    Owner,
    ItemId,
}

impl SearchField {
    fn label(self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Owner => "Owner",
            Self::ItemId => "Item ID",
        }
    }
}

pub struct AgolGui {
    events: UnboundedReceiver<LoadEvent>,
    loading: bool,
    status: String,
    error: Option<String>,
    data: Option<LoadedData>,
    view: View,
    search_field: SearchField,
    query: String,
    zero_references_only: bool,
    selected_id: Option<String>,
    graph: ConnectionGraph,
}

impl AgolGui {
    pub fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_style(&context.egui_ctx);
        let (sender, events) = unbounded_channel();
        spawn_loader(sender);
        Self {
            events,
            loading: true,
            status: "Starting…".to_string(),
            error: None,
            data: None,
            view: View::Content,
            search_field: SearchField::Title,
            query: String::new(),
            zero_references_only: false,
            selected_id: None,
            graph: ConnectionGraph::default(),
        }
    }

    fn receive_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                LoadEvent::Status(status) => self.status = status,
                LoadEvent::Ready(data) => {
                    self.selected_id = data.items.first().map(|item| item.id.clone());
                    self.graph = ConnectionGraph::from_data(&data.items, &data.references);
                    self.data = Some(*data);
                    self.loading = false;
                    self.error = None;
                    self.status = "Ready".to_string();
                }
                LoadEvent::Failed(error) => {
                    self.loading = false;
                    self.error = Some(error);
                }
            }
        }
    }

    fn restart(&mut self) {
        let (sender, events) = unbounded_channel();
        self.events = events;
        self.loading = true;
        self.error = None;
        self.status = "Starting…".to_string();
        spawn_loader(sender);
    }

    fn navigation(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("navigation").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("AGOL GUI");
                ui.separator();
                ui.selectable_value(&mut self.view, View::Content, "Content");
                ui.selectable_value(
                    &mut self.view,
                    View::Connections,
                    format!("Connections ({})", self.graph.nodes.len()),
                );
                ui.selectable_value(&mut self.view, View::Owners, "Owners");

                let broken_count = self
                    .data
                    .as_ref()
                    .map(|data| data.references.broken_connections.len())
                    .unwrap_or_default();
                ui.selectable_value(
                    &mut self.view,
                    View::BrokenConnections,
                    format!("Broken connections ({broken_count})"),
                );

                let issue_count = self
                    .data
                    .as_ref()
                    .map(|data| data.structure_issues.len())
                    .unwrap_or_default();
                ui.selectable_value(
                    &mut self.view,
                    View::StructureIssues,
                    format!("Structure issues ({issue_count})"),
                );

                if let Some(data) = &self.data {
                    ui.separator();
                    ui.label(RichText::new(&data.org.full_url).color(Color32::GRAY));
                }
            });
        });
    }

    fn loading_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(180.0);
                ui.spinner();
                ui.add_space(12.0);
                ui.heading(&self.status);
                ui.label("AGOL requests are running in the background.");
            });
        });
        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn error_screen(&mut self, ctx: &egui::Context, error: &str) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(140.0);
                ui.heading(RichText::new("Could not load ArcGIS Online").color(Color32::RED));
                ui.add_space(8.0);
                ui.label(error);
                ui.add_space(16.0);
                if ui.button("Retry").clicked() {
                    self.restart();
                }
                ui.add_space(8.0);
                ui.small(
                    "Confirm ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID and \
                     ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET are set.",
                );
            });
        });
    }

    fn content_view(&mut self, ctx: &egui::Context) {
        let Some(data) = &self.data else { return };
        let visible_ids: Vec<String> = filtered_items(
            &data.items,
            &data.references,
            self.search_field,
            &self.query,
            self.zero_references_only,
        )
        .into_iter()
        .map(|item| item.id.clone())
        .collect();

        egui::TopBottomPanel::top("content_filters").show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("search_field")
                    .selected_text(self.search_field.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.search_field, SearchField::Title, "Title");
                        ui.selectable_value(&mut self.search_field, SearchField::Owner, "Owner");
                        ui.selectable_value(&mut self.search_field, SearchField::ItemId, "Item ID");
                    });
                ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("Filter organization content…")
                        .desired_width(300.0),
                );
                ui.checkbox(&mut self.zero_references_only, "Zero references only");
                if ui.button("Clear").clicked() {
                    self.query.clear();
                    self.zero_references_only = false;
                }
                ui.separator();
                ui.label(format!(
                    "{} of {} items",
                    visible_ids.len(),
                    data.items.len()
                ));
            });
        });

        egui::SidePanel::left("content_list")
            .resizable(true)
            .default_width(430.0)
            .width_range(280.0..=650.0)
            .show(ctx, |ui| {
                ui.heading("Organization content");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for id in &visible_ids {
                        let Some(item) = data.items.iter().find(|item| &item.id == id) else {
                            continue;
                        };
                        let selected = self.selected_id.as_deref() == Some(item.id.as_str());
                        let response = ui.selectable_label(
                            selected,
                            format!("{}\n{} · {}", item.title, item.item_type, item.owner),
                        );
                        if response.clicked() {
                            self.selected_id = Some(item.id.clone());
                        }
                        response.on_hover_text(&item.id);
                        ui.separator();
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self
                .selected_id
                .as_deref()
                .and_then(|id| data.items.iter().find(|item| item.id == id))
            {
                Some(item) => {
                    item_details(ui, item, &data.org.full_url, &data.items, &data.references)
                }
                None => {
                    ui.centered_and_justified(|ui| ui.label("Select an item to view its details."));
                }
            }
        });
    }

    fn owners_view(&self, ctx: &egui::Context) {
        let Some(data) = &self.data else { return };
        let counts = owner_counts(&data.items);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Content by owner ({} owners)", counts.len()));
            ui.label(format!(
                "{} members returned by the organization directory",
                data.users.len()
            ));
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("owners_grid")
                    .striped(true)
                    .min_col_width(220.0)
                    .show(ui, |ui| {
                        ui.strong("Owner");
                        ui.strong("Items");
                        ui.end_row();
                        for (owner, count) in counts {
                            ui.label(owner);
                            ui.label(count.to_string());
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn connections_view(&mut self, ctx: &egui::Context) {
        let Some(data) = &self.data else { return };

        egui::TopBottomPanel::top("graph_controls").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Item connections");
                ui.separator();
                if ui
                    .button(if self.graph.simulating {
                        "Pause layout"
                    } else {
                        "Resume layout"
                    })
                    .clicked()
                {
                    self.graph.simulating = !self.graph.simulating;
                }
                if ui.button("Reset layout").clicked() {
                    self.graph.reset_layout();
                }
                if ui.button("Fit graph").clicked() {
                    self.graph.fit_requested = true;
                }
                ui.separator();
                graph_legend(ui, NodeKind::FeatureLayer);
                graph_legend(ui, NodeKind::WebMap);
                graph_legend(ui, NodeKind::WebApplication);
            });
        });

        if let Some(selected) = self.graph.selected {
            let node = self.graph.nodes[selected].clone();
            let incoming: Vec<_> = self
                .graph
                .incoming(selected)
                .map(|index| (index, self.graph.nodes[index].clone()))
                .collect();
            let outgoing: Vec<_> = self
                .graph
                .outgoing(selected)
                .map(|index| (index, self.graph.nodes[index].clone()))
                .collect();
            egui::SidePanel::right("graph_details")
                .resizable(true)
                .default_width(280.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(&node.title);
                        ui.label(RichText::new(node.kind.label()).color(node.kind.color()));
                        ui.label(&node.item_type);
                        ui.label(&node.owner);
                        ui.add_space(8.0);
                        ui.monospace(&node.id);
                        ui.add_space(8.0);
                        ui.label(format!(
                            "{} incoming · {} outgoing",
                            incoming.len(),
                            outgoing.len()
                        ));
                        if ui.button("Open in ArcGIS Online").clicked() {
                            let _ = webbrowser::open(&format!(
                                "{}/home/item.html?id={}",
                                data.org.full_url, node.id
                            ));
                        }

                        ui.add_space(16.0);
                        graph_neighbor_list(
                            ui,
                            "Incoming",
                            "Items this node uses",
                            &incoming,
                            &mut self.graph.selected,
                        );
                        ui.add_space(12.0);
                        graph_neighbor_list(
                            ui,
                            "Outgoing",
                            "Items that use this node",
                            &outgoing,
                            &mut self.graph.selected,
                        );

                        ui.add_space(16.0);
                        if ui.button("Close details").clicked() {
                            self.graph.selected = None;
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.graph.show(ui);
        });
        if self.graph.simulating {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn broken_connections_view(&self, ctx: &egui::Context) {
        let Some(data) = &self.data else { return };
        let mut items: Vec<_> = data.references.broken_connections.iter().collect();
        items.sort_by(|left, right| left.title.cmp(&right.title));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Broken connections ({})", items.len()));
            ui.label("Items that refer to a source item no longer present in the organization.");
            ui.separator();
            item_table(ui, items.into_iter(), &data.org.full_url);
        });
    }

    fn structure_issues_view(&self, ctx: &egui::Context) {
        let Some(data) = &self.data else { return };
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!(
                "Unexpected item structures ({})",
                data.structure_issues.len()
            ));
            ui.label("These items could not be analyzed using the expected schema.");
            ui.separator();
            issue_table(ui, &data.structure_issues, &data.org.full_url);
        });
    }
}

impl eframe::App for AgolGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_events();
        self.navigation(ctx);

        if self.loading {
            self.loading_screen(ctx);
            return;
        }
        if let Some(error) = self.error.clone() {
            self.error_screen(ctx, &error);
            return;
        }

        match self.view {
            View::Content => self.content_view(ctx),
            View::Connections => self.connections_view(ctx),
            View::Owners => self.owners_view(ctx),
            View::BrokenConnections => self.broken_connections_view(ctx),
            View::StructureIssues => self.structure_issues_view(ctx),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum NodeKind {
    FeatureLayer,
    WebMap,
    WebApplication,
}

impl NodeKind {
    fn from_item_type(item_type: &str) -> Option<Self> {
        match item_type {
            "Feature Service" | "Feature Collection" => Some(Self::FeatureLayer),
            "Web Map" => Some(Self::WebMap),
            "Web Mapping Application"
            | "Dashboard"
            | "Web Experience"
            | "Web Experience Template"
            | "Hub Site Application"
            | "Application"
            | "Form"
            | "Solution"
            | "Hub Page"
            | "Notebook" => Some(Self::WebApplication),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::FeatureLayer => "Feature layer/service",
            Self::WebMap => "Web map",
            Self::WebApplication => "Web application",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::FeatureLayer => Color32::from_rgb(66, 165, 245),
            Self::WebMap => Color32::from_rgb(102, 187, 106),
            Self::WebApplication => Color32::from_rgb(255, 167, 38),
        }
    }

    fn anchor_x(self) -> f32 {
        match self {
            Self::FeatureLayer => -360.0,
            Self::WebMap => 0.0,
            Self::WebApplication => 360.0,
        }
    }
}

#[derive(Debug, Clone)]
struct GraphNode {
    id: String,
    title: String,
    item_type: String,
    owner: String,
    kind: NodeKind,
    position: egui::Vec2,
    velocity: egui::Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GraphEdge {
    from: usize,
    to: usize,
}

#[derive(Debug)]
struct ConnectionGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    pan: egui::Vec2,
    zoom: f32,
    selected: Option<usize>,
    dragged_node: Option<usize>,
    simulating: bool,
    layout_iterations: usize,
    fit_requested: bool,
}

impl Default for ConnectionGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            selected: None,
            dragged_node: None,
            simulating: true,
            layout_iterations: 0,
            fit_requested: true,
        }
    }
}

impl ConnectionGraph {
    fn from_data(items: &[ArcGISSearchResults], references: &ArcGISReferences) -> Self {
        let items_by_id: HashMap<_, _> =
            items.iter().map(|item| (item.id.as_str(), item)).collect();
        let mut raw_edges = HashSet::new();
        let mut connected_ids: HashSet<_> = items
            .iter()
            .filter(|item| NodeKind::from_item_type(&item.item_type).is_some())
            .map(|item| item.id.clone())
            .collect();

        for (source_id, dependents) in &references.lookup {
            let Some(source) = items_by_id.get(source_id.as_str()) else {
                continue;
            };
            let Some(source_kind) = NodeKind::from_item_type(&source.item_type) else {
                continue;
            };
            for dependent in dependents {
                let Some(target) = items_by_id.get(dependent.id.as_str()) else {
                    continue;
                };
                let Some(target_kind) = NodeKind::from_item_type(&target.item_type) else {
                    continue;
                };
                let is_requested_connection = matches!(
                    (source_kind, target_kind),
                    (NodeKind::FeatureLayer, NodeKind::WebMap)
                        | (NodeKind::WebMap, NodeKind::WebApplication)
                );
                if is_requested_connection {
                    raw_edges.insert((source.id.clone(), target.id.clone()));
                    connected_ids.insert(source.id.clone());
                    connected_ids.insert(target.id.clone());
                }
            }
        }

        let mut connected_items: Vec<_> = connected_ids
            .iter()
            .filter_map(|id| items_by_id.get(id.as_str()).copied())
            .collect();
        connected_items.sort_by(|left, right| {
            NodeKind::from_item_type(&left.item_type)
                .cmp(&NodeKind::from_item_type(&right.item_type))
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });

        let kind_counts = connected_items
            .iter()
            .fold(HashMap::new(), |mut counts, item| {
                if let Some(kind) = NodeKind::from_item_type(&item.item_type) {
                    *counts.entry(kind).or_insert(0usize) += 1;
                }
                counts
            });
        let mut kind_indices = HashMap::new();
        let nodes: Vec<_> = connected_items
            .into_iter()
            .filter_map(|item| {
                let kind = NodeKind::from_item_type(&item.item_type)?;
                let index = kind_indices.entry(kind).or_insert(0usize);
                let count = *kind_counts.get(&kind).unwrap_or(&1);
                let y = (*index as f32 - (count.saturating_sub(1) as f32 / 2.0)) * 62.0;
                *index += 1;
                Some(GraphNode {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    item_type: item.item_type.clone(),
                    owner: item.owner.clone(),
                    kind,
                    position: egui::vec2(kind.anchor_x(), y),
                    velocity: egui::Vec2::ZERO,
                })
            })
            .collect();

        let indices: HashMap<_, _> = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        let mut edges: Vec<_> = raw_edges
            .into_iter()
            .filter_map(|(from, to)| {
                Some(GraphEdge {
                    from: *indices.get(from.as_str())?,
                    to: *indices.get(to.as_str())?,
                })
            })
            .collect();
        edges.sort_by_key(|edge| (edge.from, edge.to));

        Self {
            nodes,
            edges,
            ..Self::default()
        }
    }

    fn reset_layout(&mut self) {
        let mut kind_indices = HashMap::new();
        let mut kind_counts = HashMap::new();
        for node in &self.nodes {
            *kind_counts.entry(node.kind).or_insert(0usize) += 1;
        }
        for node in &mut self.nodes {
            let index = kind_indices.entry(node.kind).or_insert(0usize);
            let count = *kind_counts.get(&node.kind).unwrap_or(&1);
            node.position = egui::vec2(
                node.kind.anchor_x(),
                (*index as f32 - (count.saturating_sub(1) as f32 / 2.0)) * 62.0,
            );
            node.velocity = egui::Vec2::ZERO;
            *index += 1;
        }
        self.pan = egui::Vec2::ZERO;
        self.zoom = 1.0;
        self.layout_iterations = 0;
        self.simulating = true;
        self.fit_requested = true;
    }

    fn incoming(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        self.edges
            .iter()
            .filter(move |edge| edge.to == node)
            .map(|edge| edge.from)
    }

    fn outgoing(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        self.edges
            .iter()
            .filter(move |edge| edge.from == node)
            .map(|edge| edge.to)
    }

    fn step_layout(&mut self) {
        if !self.simulating || self.nodes.is_empty() {
            return;
        }
        let mut forces = vec![egui::Vec2::ZERO; self.nodes.len()];

        if self.nodes.len() <= 450 {
            for left in 0..self.nodes.len() {
                for right in (left + 1)..self.nodes.len() {
                    let delta = self.nodes[left].position - self.nodes[right].position;
                    let distance_sq = delta.length_sq().max(100.0);
                    let direction = delta.normalized();
                    let force = direction * (1800.0 / distance_sq);
                    forces[left] += force;
                    forces[right] -= force;
                }
            }
        }

        for edge in &self.edges {
            let delta = self.nodes[edge.to].position - self.nodes[edge.from].position;
            let distance = delta.length().max(1.0);
            let force = delta / distance * ((distance - 145.0) * 0.008);
            forces[edge.from] += force;
            forces[edge.to] -= force;
        }

        for (index, node) in self.nodes.iter().enumerate() {
            forces[index].x += (node.kind.anchor_x() - node.position.x) * 0.004;
            forces[index].y += -node.position.y * 0.0004;
        }
        for (node, force) in self.nodes.iter_mut().zip(forces) {
            let velocity = node.velocity + force;
            node.velocity = if velocity.length() > 8.0 {
                velocity.normalized() * 8.0
            } else {
                velocity
            } * 0.84;
            node.position += node.velocity;
        }

        self.layout_iterations += 1;
        if self.layout_iterations >= 500 {
            self.simulating = false;
        }
    }

    fn fit_to_rect(&mut self, rect: egui::Rect) {
        if self.nodes.is_empty() {
            return;
        }
        let min = self
            .nodes
            .iter()
            .fold(egui::pos2(f32::INFINITY, f32::INFINITY), |min, node| {
                min.min(node.position.to_pos2())
            });
        let max = self.nodes.iter().fold(
            egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY),
            |max, node| max.max(node.position.to_pos2()),
        );
        let graph_size = (max - min).max(egui::vec2(1.0, 1.0));
        self.zoom = ((rect.width() - 80.0) / graph_size.x)
            .min((rect.height() - 80.0) / graph_size.y)
            .clamp(0.15, 2.5);
        let graph_center = min + graph_size * 0.5;
        self.pan = -graph_center.to_vec2() * self.zoom;
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        self.step_layout();
        let available = ui.available_size().max(egui::vec2(100.0, 100.0));
        let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0.0, Color32::from_rgb(19, 22, 27));

        if self.fit_requested {
            self.fit_to_rect(rect);
            self.fit_requested = false;
        }

        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.1, 4.0);
            }
        }

        let hover_pan = self.pan;
        let hover_zoom = self.zoom;
        let to_hover_screen =
            |position: egui::Vec2| rect.center() + hover_pan + position * hover_zoom;
        let pointer = response.interact_pointer_pos();
        let hovered_node = pointer.and_then(|pointer| {
            self.nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    let distance = pointer.distance(to_hover_screen(node.position));
                    (distance <= 12.0).then_some((index, distance))
                })
                .min_by(|left, right| left.1.total_cmp(&right.1))
                .map(|(index, _)| index)
        });

        if response.drag_started() {
            self.dragged_node = hovered_node;
        }
        if response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            if let Some(index) = self.dragged_node {
                self.nodes[index].position += delta / self.zoom;
                self.nodes[index].velocity = egui::Vec2::ZERO;
            } else {
                self.pan += delta;
            }
        }
        if response.drag_stopped() {
            self.dragged_node = None;
        }
        if response.clicked() {
            self.selected = hovered_node;
        }

        let paint_pan = self.pan;
        let paint_zoom = self.zoom;
        let to_screen = |position: egui::Vec2| rect.center() + paint_pan + position * paint_zoom;

        for edge in &self.edges {
            painter.line_segment(
                [
                    to_screen(self.nodes[edge.from].position),
                    to_screen(self.nodes[edge.to].position),
                ],
                egui::Stroke::new(1.0, Color32::from_white_alpha(65)),
            );
        }
        for (index, node) in self.nodes.iter().enumerate() {
            let center = to_screen(node.position);
            let is_selected = self.selected == Some(index);
            let is_hovered = hovered_node == Some(index);
            let radius = if is_selected || is_hovered { 9.0 } else { 6.0 };
            if is_selected {
                painter.circle_stroke(center, radius + 4.0, egui::Stroke::new(2.0, Color32::WHITE));
            }
            painter.circle_filled(center, radius, node.kind.color());
            if is_hovered || is_selected || (self.zoom >= 0.8 && self.nodes.len() <= 180) {
                painter.text(
                    center + egui::vec2(radius + 5.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &node.title,
                    egui::FontId::proportional(12.0),
                    Color32::from_gray(220),
                );
            }
        }

        if self.nodes.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No feature layer → web map → application connections were found.",
                egui::FontId::proportional(16.0),
                Color32::GRAY,
            );
        } else if let Some(index) = hovered_node {
            let node = &self.nodes[index];
            egui::Tooltip::for_widget(&response).show(|ui| {
                ui.strong(&node.title);
                ui.label(format!("{} · {}", node.kind.label(), node.owner));
                ui.monospace(&node.id);
            });
        }
    }
}

fn graph_legend(ui: &mut egui::Ui, kind: NodeKind) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, kind.color());
    ui.label(kind.label());
}

fn graph_neighbor_list(
    ui: &mut egui::Ui,
    heading: &str,
    description: &str,
    neighbors: &[(usize, GraphNode)],
    selected: &mut Option<usize>,
) {
    ui.heading(format!("{heading} ({})", neighbors.len()));
    ui.small(description);
    if neighbors.is_empty() {
        ui.label(RichText::new("None").italics().color(Color32::GRAY));
        return;
    }

    for (index, node) in neighbors {
        ui.horizontal_wrapped(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(rect.center(), 4.0, node.kind.color());
            if ui.link(&node.title).clicked() {
                *selected = Some(*index);
            }
        });
        ui.small(format!("{} · {}", node.item_type, node.owner));
    }
}

fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.visuals.selection.bg_fill = Color32::from_rgb(33, 92, 135);
    ctx.set_style(style);
}

fn filtered_items<'a>(
    items: &'a [ArcGISSearchResults],
    references: &ArcGISReferences,
    field: SearchField,
    query: &str,
    zero_references_only: bool,
) -> Vec<&'a ArcGISSearchResults> {
    let query = query.to_lowercase();
    let mut filtered: Vec<_> = items
        .iter()
        .filter(|item| {
            let value = match field {
                SearchField::Title => &item.title,
                SearchField::Owner => &item.owner,
                SearchField::ItemId => &item.id,
            };
            value.to_lowercase().contains(&query)
        })
        .filter(|item| {
            !zero_references_only
                || (item.item_type != "Service Definition"
                    && references
                        .lookup
                        .get(&item.id)
                        .is_some_and(HashSet::is_empty))
        })
        .collect();
    filtered.sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));
    filtered
}

fn item_details(
    ui: &mut egui::Ui,
    item: &ArcGISSearchResults,
    org_url: &str,
    all_items: &[ArcGISSearchResults],
    references: &ArcGISReferences,
) {
    ui.heading(&item.title);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&item.item_type).strong());
        ui.label("·");
        ui.label(&item.owner);
        ui.label("·");
        ui.label(&item.access);
    });
    ui.add_space(8.0);
    ui.monospace(&item.id);
    if ui.button("Open in ArcGIS Online").clicked() {
        let _ = webbrowser::open(&format!("{org_url}/home/item.html?id={}", item.id));
    }

    if let Some(snippet) = &item.snippet {
        ui.add_space(12.0);
        ui.label(snippet);
    }

    ui.add_space(18.0);
    let mut incoming: Vec<_> = references
        .lookup
        .iter()
        .filter(|(_, dependents)| dependents.iter().any(|dependent| dependent.id == item.id))
        .filter_map(|(source_id, _)| {
            all_items
                .iter()
                .find(|candidate| candidate.id == *source_id)
        })
        .collect();
    incoming.sort_by(|left, right| left.title.cmp(&right.title));

    let mut outgoing: Vec<_> = references
        .lookup
        .get(&item.id)
        .map(|items| items.iter().collect())
        .unwrap_or_default();
    outgoing.sort_by(|left, right| left.title.cmp(&right.title));

    ui.heading("Connections");
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        item_reference_list(
            ui,
            "Uses / incoming",
            "Items referenced by this item",
            &incoming,
            org_url,
        );
        ui.add_space(14.0);
        item_reference_list(
            ui,
            "Used by / outgoing",
            "Items that reference this item",
            &outgoing,
            org_url,
        );
    });
}

fn item_reference_list(
    ui: &mut egui::Ui,
    heading: &str,
    description: &str,
    items: &[&ArcGISSearchResults],
    org_url: &str,
) {
    ui.strong(format!("{heading} ({})", items.len()));
    ui.small(description);
    if items.is_empty() {
        ui.label(RichText::new("None").italics().color(Color32::GRAY));
        return;
    }

    for referenced in items {
        ui.horizontal_wrapped(|ui| {
            if ui.link(&referenced.title).clicked() {
                let _ = webbrowser::open(&format!("{org_url}/home/item.html?id={}", referenced.id));
            }
            ui.label(format!("{} · {}", referenced.item_type, referenced.owner));
        });
    }
}

fn item_table<'a>(
    ui: &mut egui::Ui,
    items: impl Iterator<Item = &'a ArcGISSearchResults>,
    org_url: &str,
) {
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("item_table")
            .striped(true)
            .min_col_width(150.0)
            .show(ui, |ui| {
                ui.strong("Title");
                ui.strong("Type");
                ui.strong("Owner");
                ui.strong("Item ID");
                ui.end_row();
                for item in items {
                    if ui.link(&item.title).clicked() {
                        let _ =
                            webbrowser::open(&format!("{org_url}/home/item.html?id={}", item.id));
                    }
                    ui.label(&item.item_type);
                    ui.label(&item.owner);
                    ui.monospace(&item.id);
                    ui.end_row();
                }
            });
    });
}

fn issue_table(ui: &mut egui::Ui, issues: &[ItemIssue], org_url: &str) {
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("issue_table")
            .striped(true)
            .min_col_width(130.0)
            .show(ui, |ui| {
                ui.strong("Title");
                ui.strong("Type");
                ui.strong("Owner");
                ui.strong("Reason");
                ui.end_row();
                for issue in issues {
                    if ui.link(&issue.item.title).clicked() {
                        let _ = webbrowser::open(&format!(
                            "{org_url}/home/item.html?id={}",
                            issue.item.id
                        ));
                    }
                    ui.label(&issue.item.item_type);
                    ui.label(&issue.item.owner);
                    ui.label(&issue.reason).on_hover_text(&issue.reason);
                    ui.end_row();
                }
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, title: &str, owner: &str, item_type: &str) -> ArcGISSearchResults {
        ArcGISSearchResults {
            id: id.to_string(),
            owner: owner.to_string(),
            org_id: "org".to_string(),
            created: 0,
            is_org_item: true,
            modified: 0,
            guid: None,
            name: None,
            title: title.to_string(),
            item_type: item_type.to_string(),
            description: None,
            tags: Vec::new(),
            snippet: None,
            url: None,
            access: "private".to_string(),
        }
    }

    #[test]
    fn filters_by_each_search_field_case_insensitively() {
        let items = [
            item("roads-id", "Road Closures", "Alice", "Web Map"),
            item("parks-id", "Parks", "Bob", "Feature Service"),
        ];
        let references = ArcGISReferences::default();

        assert_eq!(
            filtered_items(&items, &references, SearchField::Title, "ROAD", false).len(),
            1
        );
        assert_eq!(
            filtered_items(&items, &references, SearchField::Owner, "bob", false)[0].id,
            "parks-id"
        );
        assert_eq!(
            filtered_items(&items, &references, SearchField::ItemId, "roads", false)[0].id,
            "roads-id"
        );
    }

    #[test]
    fn zero_reference_filter_excludes_service_definitions() {
        let items = [
            item("empty", "Empty", "Alice", "Feature Service"),
            item("used", "Used", "Alice", "Feature Service"),
            item("source", "Source", "Alice", "Service Definition"),
        ];
        let mut references = ArcGISReferences::default();
        references
            .lookup
            .insert("empty".to_string(), HashSet::new());
        references
            .lookup
            .insert("used".to_string(), HashSet::from([items[0].clone()]));
        references
            .lookup
            .insert("source".to_string(), HashSet::new());

        let filtered = filtered_items(&items, &references, SearchField::Title, "", true);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "empty");
    }

    #[test]
    fn connection_graph_builds_feature_map_application_chain() {
        let items = [
            item("layer", "Layer", "Alice", "Feature Service"),
            item("orphan", "Unreferenced layer", "Alice", "Feature Service"),
            item("map", "Map", "Alice", "Web Map"),
            item("app", "App", "Alice", "Web Mapping Application"),
            item("pdf", "Document", "Alice", "PDF"),
        ];
        let mut references = ArcGISReferences::default();
        references
            .lookup
            .insert("layer".to_string(), HashSet::from([items[2].clone()]));
        references
            .lookup
            .insert("map".to_string(), HashSet::from([items[3].clone()]));

        let graph = ConnectionGraph::from_data(&items, &references);

        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.nodes.iter().all(|node| node.id != "pdf"));

        let map_index = graph
            .nodes
            .iter()
            .position(|node| node.id == "map")
            .unwrap();
        let incoming: Vec<_> = graph
            .incoming(map_index)
            .map(|index| graph.nodes[index].id.as_str())
            .collect();
        let outgoing: Vec<_> = graph
            .outgoing(map_index)
            .map(|index| graph.nodes[index].id.as_str())
            .collect();
        assert_eq!(incoming, ["layer"]);
        assert_eq!(outgoing, ["app"]);
    }
}
