use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Binding;
use kube::api::PostParams;
use kube::api::WatchEvent;
use kube::api::WatchParams;
use kube::ResourceExt;
use kube::{Api, Client};

use k8s_openapi::api::core::v1::Pod;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建 Kubernetes 客户端
    let client = Client::try_default().await?;

    //
    let k8s_scheduler_rs = "my-scheduler";

    // 获取 Pod 和 Node 的 API 对象
    let pods: Api<Pod> = Api::all(client.clone());

    let watch_fileds = "status.phase=Pending,spec.schedulerName=".to_string() + k8s_scheduler_rs;
    let watch_params = WatchParams::default().fields(&watch_fileds);

    // 监听未调度的 Pod
    let mut stream = pods.watch(&watch_params, "0").await?.boxed();

    let node_name = "xx";
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(p) => {
                let ns = p.namespace().unwrap();
                let name = p.name_any();
                println!("Added: name={:?}, namespace={:?}", name, ns);

                let pod_bind = Api::<Pod>::namespaced(client.clone(), &ns);

                let binding = Binding {
                    metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                        name: Some(name.clone()),
                        ..Default::default()
                    },
                    target: k8s_openapi::api::core::v1::ObjectReference {
                        kind: Some("Node".to_string()),
                        name: Some(node_name.to_string()),
                        ..Default::default()
                    },
                };

                let res: Result<Pod, kube::Error> = pod_bind
                    .create_subresource::<Pod>(
                        "binding",
                        &name,
                        &PostParams::default(),
                        serde_json::to_vec(&binding)?,
                    )
                    .await;
                match res {
                    Ok(_p) => {
                        println!("Successfully assigned Pod {} to Node {}", name, node_name);
                    }
                    Err(e) => {
                        println!("Failed to assign Pod {} to Node {}: {}", name, node_name, e);
                    }
                }
                println!("Successfully assigned Pod {} to Node {}", name, node_name,);
            }
            _ => (),
        }
    }

    Ok(())
}
