use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Binding;
use k8s_openapi::api::core::v1::Node;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Status;
use kube::ResourceExt;
use kube::api::ListParams;
use kube::api::PostParams;
use kube::api::WatchEvent;
use kube::api::WatchParams;
use kube::{Api, Client};
use tracing_subscriber::EnvFilter;

use k8s_openapi::api::core::v1::Pod;
use rand::prelude::*;

#[derive(Clone, Debug)]
struct SchedulerConfig {
    scheduler_name: String,
    phase_pending: String,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            scheduler_name: "my-scheduler-rs".to_string(),
            phase_pending: "Pending".to_string(),
        }
    }
}

impl SchedulerConfig {
    pub fn new(scheduler_name: String, phase_pending: String) -> Self {
        Self {
            scheduler_name,
            phase_pending,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = clap::Command::new("my-scheduler-rs")
        .version("0.1")
        .about("k8s scheduler impl in rust")
        .arg(
            clap::Arg::new("scheduler-name")
                .long("name")
                .short('n')
                .default_value("my-scheduler")
                .help("Name of the scheduler"),
        )
        .arg(
            clap::Arg::new("node-label-selector")
                .short('l')
                .long("label")
                .default_value("my-sheduler-node=test-1")
                .help("Node label selector"),
        )
        .get_matches();

    let scheduler_name = matches.get_one::<String>("scheduler-name").unwrap();
    let node_labe = matches.get_one::<String>("node-label-selector").unwrap();

    let config = SchedulerConfig::new(scheduler_name.to_owned(), "Pending".to_owned());
    let node_label_selector = node_labe.to_owned();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::all(client.clone());

    let watch_fileds = format!(
        "status.phase={},spec.schedulerName={}",
        config.phase_pending, config.scheduler_name
    );
    let watch_params = WatchParams::default().fields(&watch_fileds);

    let mut stream = pods.watch(&watch_params, "0").await?.boxed();

    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(p) => {
                let ns = p.namespace().unwrap_or_default();
                let name = p.name_any();
                println!("pod = {}, namespace = {}, Found pending pod", name, ns);

                let nodes = get_node_names(client.clone(), &node_label_selector).await?;
                let node_name = get_better_node_name(nodes);

                println!(
                    "pod = {}, namespace = {}, node = {}, Assigning pod to node",
                    name, ns, node_name
                );

                let binding = create_binding(&name, &node_name);

                let pod_bind = Api::<Pod>::namespaced(client.clone(), &ns);
                match bind_pod(&pod_bind, &name, &binding).await {
                    Ok(s) => {
                        println!(
                            "Successfully assigned Pod {} to Node {}, status={}",
                            name,
                            node_name,
                            s.status.unwrap()
                        );
                    }
                    Err(e) => {
                        println!(
                            "Failed to assign Pod {} to Node {}: error: {}",
                            name, node_name, e
                        );
                    }
                }
            }
            _ => {
                tracing::info!("Other event: {:?}, do nothing", status);
            }
        }
    }

    Ok(())
}

fn create_binding(name: &str, node_name: &str) -> Binding {
    Binding {
        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        },
        target: k8s_openapi::api::core::v1::ObjectReference {
            kind: Some("Node".to_string()),
            name: Some(node_name.to_string()),
            ..Default::default()
        },
    }
}

async fn bind_pod(
    pod_api: &Api<Pod>,
    name: &str,
    binding: &Binding,
) -> Result<Status, kube::Error> {
    pod_api
        .create_subresource(
            "binding",
            name,
            &PostParams::default(),
            serde_json::to_vec(binding).unwrap(),
        )
        .await
}

async fn get_node_names(
    client: Client,
    label_selector: &str,
) -> anyhow::Result<Vec<String>, anyhow::Error> {
    let mut nodes_res = Vec::new();
    let nodes: Api<Node> = Api::all(client);

    let list_p = ListParams::default().labels(label_selector);

    for node in nodes.list(&list_p).await?.items {
        nodes_res.push(node.metadata.name.unwrap());
    }

    Ok(nodes_res)
}

fn get_better_node_name(nodes: Vec<String>) -> String {
    let mut rng = rand::rng();
    nodes.choose(&mut rng).unwrap().to_owned()
}
