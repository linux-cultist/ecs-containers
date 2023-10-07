use std::error::Error;
use aws_sdk_ecs::Client;
use aws_types::region::Region;
use futures::future;
use tokio::task::JoinHandle;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let regions = vec!["eu-west-1", "eu-central-1"];

    let handles: Vec<JoinHandle<Vec<String>>> = regions
        .iter()
        .cloned()
        .map(|region| tokio::task::spawn(fetch_containers_for_region(&region)))
        .collect();

    let results: Vec<String> = future::join_all(handles)
        .await
        .into_iter()
        .filter_map(Result::ok)  // filter out the errors
        .flat_map(|vec| vec.into_iter())
        .collect();

    print_containers(results);

    Ok(())


}

async fn fetch_containers_for_region(region_str: &str) -> Vec<String> {
    println!("Fetching containers for region: {}", region_str);
    let region = Region::new(region_str.to_string());
    let shared_config = aws_config::from_env().region(region.clone()).load().await;
    let client = Client::new(&shared_config);
    let mut result = Vec::new();

    let resp = client.list_clusters().send().await.unwrap();
    let clusters = client
        .describe_clusters()
        .set_clusters(resp.cluster_arns)
        .send()
        .await.unwrap();

    for cluster in clusters.clusters().unwrap() {
        // List tasks for the current cluster
        let tasks = client
            .list_tasks()
            .set_cluster(cluster.cluster_name.clone())
            .send()
            .await.unwrap();

        let task_arns = tasks.task_arns().unwrap_or_default();
        if !task_arns.is_empty() {
            let task_details = client
                .describe_tasks()
                .set_tasks(Some(task_arns.into()))
                .set_cluster(cluster.cluster_name.clone())
                .send()
                .await.unwrap();

            for task in task_details.tasks().unwrap_or_default() {
                for container in task.containers().unwrap_or_default() {
                    let container_name = container.name().unwrap_or_default();
                    let cluster_name = cluster.cluster_name().unwrap_or_default();
                    let container_arn = container.task_arn().unwrap_or_default();
                    let container_id = container_arn.rsplit('/').next().unwrap_or("");
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
    result


}


pub fn print_containers(containers: Vec<String>) {
    println!(
        "{:<20}{:<30}{:>8}{:>41}",
        "Region", "Cluster", "Container", "Container ID"
    );
    println!("--------------------------------------------------------------------------------------------------------------------");

    for container in containers {
        println!("{}", container);
    }

}
