use aws_sdk_ecs::Client;
use aws_types::region::Region;
use tokio::task::{JoinHandle, spawn};
use anyhow::Result;

#[tokio::main]
async fn main() {
    let regions = vec!["eu-west-1", "eu-central-1"];
    let handles: Vec<JoinHandle<anyhow::Result<Vec<String>>>> = regions
        .iter()
        .cloned()
        .map(|region| spawn(fetch_containers(&region)))
        .collect();

    print_result_header();

    for handle in handles {
        match handle.await {
            Ok(results) => {
                if let Ok(containers) = results {
                    containers
                        .iter()
                        .for_each(|container| println!("{}", container));
                }
            },
            Err(e) => { println!("{}", e); },
        }
    }
}

async fn fetch_containers(region_str: &str) -> Result<Vec<String>> {
    let region = Region::new(region_str.to_string());
    let shared_config = aws_config::from_env().region(region.clone()).load().await;
    let client = Client::new(&shared_config);
    let mut result = Vec::new();

    let resp = client.list_clusters().send().await?;

    let clusters = client
        .describe_clusters()
        .set_clusters(resp.cluster_arns)
        .send()
        .await?;

    for cluster in clusters.clusters().ok_or(anyhow::anyhow!("No clusters found"))? {
        // List tasks for the current cluster
        let tasks = client
            .list_tasks()
            .set_cluster(cluster.cluster_name.clone())
            .send()
            .await?;

        let task_arns = tasks.task_arns().ok_or(anyhow::anyhow!("No tasks found."))?;
        if !task_arns.is_empty() {
            let task_details = client
                .describe_tasks()
                .set_tasks(Some(task_arns.into()))
                .set_cluster(cluster.cluster_name.clone())
                .send()
                .await?;

            for task in task_details.tasks().ok_or(anyhow::anyhow!("No task details found."))? {
                for container in task.containers().unwrap_or_default() {
                    let container_name = container.name().unwrap_or_default();
                    let cluster_name = cluster.cluster_name().unwrap_or_default();
                    let container_arn = container.task_arn().unwrap_or_default();
                    let container_id = container_arn.rsplit('/').next().unwrap_or_default();
                    if !container_name.starts_with("ecs-service-connect") {
                        let formatted_string = format!(
                            "{:<20}{:<30}{:<30}{:>40}",
                            region.to_string(),
                            cluster_name,
                            container_name,
                            container_id
                        );
                        result.push(formatted_string);
                    }
                }
            }
        }
    }
    Ok(result)


}

fn print_result_header() {
    println!(
        "{:<20}{:<30}{:>8}{:>41}",
        "Region", "Cluster", "Container", "Container ID"
    );
    println!("--------------------------------------------------------------------------------------------------------------------");
}

