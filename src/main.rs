use std::{sync::mpsc, thread};

use aws_sdk_ecs::Client;
use aws_types::region::Region;
use tokio::task::{self};

#[derive(Debug)]
pub enum MyError {
    Aws(aws_sdk_ecs::Error),
    Task(tokio::task::JoinError),
}

impl From<aws_sdk_ecs::Error> for MyError {
    fn from(err: aws_sdk_ecs::Error) -> Self {
        MyError::Aws(err)
    }
}

impl From<tokio::task::JoinError> for MyError {
    fn from(err: tokio::task::JoinError) -> Self {
        MyError::Task(err)
    }
}

#[tokio::main]
async fn main() -> Result<(), MyError> {
    let mut handles: Vec<task::JoinHandle<Result<Vec<String>, aws_sdk_ecs::Error>>> = Vec::new();
    let regions = vec!["eu-west-1", "eu-central-1"];

    for region_name in &regions {
        let region_name = region_name.to_string();
        let handle = task::spawn(async move {
            let region = Region::new(region_name);
            let shared_config = aws_config::from_env().region(region.clone()).load().await;
            let client = Client::new(&shared_config);
            list_containers(&client, region).await
        });

        handles.push(handle);
    }

    print_result_headers();

    for handle in handles {
        handle.await??.iter().for_each(|line| println!("{}", line));
    }

    Ok(())
}

async fn list_containers(
    client: &Client,
    region: Region,
) -> Result<Vec<String>, aws_sdk_ecs::Error> {
    let mut result = Vec::new();
    let resp = client.list_clusters().send().await?;
    let clusters = client
        .describe_clusters()
        .set_clusters(resp.cluster_arns)
        .send()
        .await?;

    for cluster in clusters.clusters().unwrap() {
        // List tasks for the current cluster
        let tasks = client
            .list_tasks()
            .set_cluster(cluster.cluster_name.clone())
            .send()
            .await?;

        let task_arns = tasks.task_arns().unwrap_or_default();
        if !task_arns.is_empty() {
            let task_details = client
                .describe_tasks()
                .set_tasks(Some(task_arns.into()))
                .set_cluster(cluster.cluster_name.clone())
                .send()
                .await?;

            for task in task_details.tasks().unwrap_or_default() {
                for container in task.containers().unwrap_or_default() {
                    let container_name = container.name().unwrap_or_default();
                    let cluster_name = cluster.cluster_name().unwrap_or_default();
                    let container_arn = container.task_arn().unwrap_or_default();
                    let container_id = container_arn.rsplit('/').next().unwrap_or("");
                    if !container_name.starts_with("ecs-service-connect") {
                        let formatted_string = format!(
                            "{:<20}{:<30}{:<26}{:>40}",
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

pub fn print_result_headers() {
    println!(
        "{:<20}{:<30}{:>8}{:>37}",
        "Region", "Cluster", "Container", "Container ID"
    );
    println!("--------------------------------------------------------------------------------------------------------------------");
}
