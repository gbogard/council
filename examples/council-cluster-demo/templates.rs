use std::collections::HashMap;

use council::{node::NodeId, ClusterEvent};
use maud::*;

use crate::application::{Application, RunningCouncil};

fn html_page_structure(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        head {
            meta charset="utf-8";
            link rel="stylesheet" href="/picnic.min.css";
            link rel="stylesheet" href="/main.css";
            title { "Council Demo App" }
        }
        body {
            (content)
        }
    }
}

fn navigation(node_ids: &[NodeId], instance: &RunningCouncil) -> Markup {
    html! {
        nav {
            div class="nav-content" {
                label { "Select a node: " }
                select onchange="location = `/?node_id=${this.value}`;" {
                    @for node_id in node_ids {
                        option selected[*node_id == instance.council_instance.this_node_id] value={(node_id.to_string())} { (node_id.unique_id )}
                    }
                }
            }

        }
    }
}

async fn render_node(node_ids: &[NodeId], instance: &RunningCouncil) -> Markup {
    let cluster = instance.council_instance.cluster().await.unwrap();
    html! {
        (navigation(node_ids, instance))
        main {
            h2 { "This node"}
            strong { "ID : " } (cluster.this_node_id.to_string())
            h2 { "Know members"}
            div class="card-grid" {
                @for member_view in cluster.cluster_view.known_members.values() {
                    div class="card" {
                        header {
                            a href={"/?node_id=" (member_view.id.to_string()) } {
                                (member_view.id.unique_id)
                                @if member_view.id == cluster.this_node_id {
                                    " (This node)"
                                }
                            }
                        }
                        ul {
                            li { strong { "Advertised URL: " } (member_view.advertised_addr.to_string()) }
                            @if let Some(state) = &member_view.state {
                                li { strong { "Version: " } (state.version) }
                                li { strong { "Status: " } (state.node_status.to_string()) }
                                li { strong { "Heartbeat: " } (state.heartbeat) }
                            }
                        }
                    }
                }
            }
            div class="flex two" {
                div {
                    h2 { "Peer Nodes"}
                    ul {
                        @for url in &cluster.peer_nodes {
                            li { (url.to_string()) }
                        }
                    }
                }
                div {
                    h2 { "Unknown Peer Nodes"}
                    ul {
                        @for url in &cluster.unknwon_peer_nodes {
                            li { (url.to_string()) }
                        }
                    }
                }
            }
            h2 { "Raw cluster state" }
            div class="card" id="raw-cluster-state" {
                pre { (serde_json::ser::to_string_pretty(&cluster).unwrap()) }
            }


        }
    }
}

fn not_found() -> Markup {
    html! {
        div {
            "Not found"
        }
    }
}

pub async fn home_page(app: &Application, chosen_id: Option<NodeId>) -> Markup {
    let node_ids: Vec<NodeId> = app.instances.keys().cloned().collect();
    let instance = chosen_id
        .and_then(|id| app.instances.get(&id))
        .or_else(|| app.instances.values().next());

    let content = match instance {
        Some(instance) => render_node(&node_ids[..], instance).await,
        None => not_found(),
    };

    html_page_structure(html! {
        (content)
    })
}
