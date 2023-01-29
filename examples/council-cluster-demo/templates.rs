use std::time::Instant;

use council::{cluster::views::MemberView, node::NodeId};
use maud::*;

use crate::application::{Application, RunningCouncil};

const UNKNOWN: &'static str = "Ø";

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
            div .nav-content {
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
    let mut members = cluster
        .cluster_view
        .known_members
        .values()
        .collect::<Vec<&MemberView>>();
    members.sort_by_key(|m| m.id);

    let now = Instant::now();
    html! {
        (navigation(node_ids, instance))
        main {
            h2 { "This node"}
            strong { "ID : " } (cluster.this_node_id.to_string())
            br;
            strong { "Has converged? : " } (cluster.has_converged().to_string())
            h2 { "Know members"}
            div .card-grid {
                @for member_view in members {
                    div .card ."this-node"[member_view.id == cluster.this_node_id]  {
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
                                li {
                                    strong { "Observed by: " }
                                    ul { @for id in &state.observed_by { li { (id.unique_id) } } }
                                }
                            }
                        }
                    }
                }
            }
            div .flex two {
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
            h2 { "Failure detector" }
            table {
                thead {
                    tr {
                        th { "Node" }
                        th { "Heartbeat" }
                        th { "Mean heartbeat time" }
                        th { "Std. dev." }
                        th { "Min" }
                        th { "Max" }
                        th { "Phi" }
                    }
                }
                tbody {
                    @for (id, m) in cluster.failure_detector.members() {
                        tr {
                            td { (id.unique_id.to_string()) }
                            td { (m.last_heartbeat) }
                            td { (m.heartbeats_intervals_mean.map_or(UNKNOWN.to_string(), |d| format!("{}ms", d.as_millis()))) }
                            td { (m.hearbeats_interval_std_dev.map_or(UNKNOWN.to_string(), |d| format!("{}ms", d.as_millis()))) }
                            td { (m.heartbeats_min_interval.map_or(UNKNOWN.to_string(), |d| format!("{}ms", d.as_millis()))) }
                            td { (m.heartbeats_max_interval.map_or(UNKNOWN.to_string(), |d| format!("{}ms", d.as_millis()))) }
                            td { (m.phi(now).map_or(UNKNOWN.to_string(), |p| p.to_string())) }
                        }
                    }
                }
            }
            h2 { "Raw cluster state (JSON)" }
            div .card id="raw-cluster-state" {
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
