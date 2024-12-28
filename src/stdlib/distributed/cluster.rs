use std::{
    sync::Arc,
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, broadcast},
    net::TcpListener,
};
use crate::{error::IoError, Result};

#[derive(Debug, Clone)]
pub struct NodeInfo {
    id: String,
    addr: SocketAddr,
    role: NodeRole,
    last_heartbeat: Instant,
    metrics: NodeMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeRole {
    Leader,
    Follower,
    Observer,
}

#[derive(Debug, Clone)]
pub struct NodeMetrics {
    cpu_usage: f64,
    memory_usage: f64,
    load_average: f64,
    active_tasks: usize,
}

pub struct ClusterManager {
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
    event_tx: broadcast::Sender<ClusterEvent>,
    config: ClusterConfig,
}

#[derive(Debug, Clone)]
pub struct ClusterConfig {
    heartbeat_interval: Duration,
    node_timeout: Duration,
    replication_factor: usize,
    min_nodes: usize,
}

#[derive(Debug, Clone)]
pub enum ClusterEvent {
    NodeJoined(NodeInfo),
    NodeLeft(String),
    LeaderElected(String),
    HealthCheck(NodeHealth),
}

impl ClusterManager {
    pub async fn new(config: ClusterConfig) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(100);
        
        let manager = Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            config,
        };

        manager.start_health_monitor();
        manager.start_leader_election();
        
        Ok(manager)
    }

    pub async fn join_cluster(&self, addr: SocketAddr) -> Result<()> {
        let node_id = generate_node_id();
        let node = NodeInfo {
            id: node_id.clone(),
            addr,
            role: NodeRole::Follower,
            last_heartbeat: Instant::now(),
            metrics: NodeMetrics::default(),
        };

        // Register node
        self.nodes.write().await.insert(node_id.clone(), node.clone());
        
        // Notify cluster
        self.event_tx.send(ClusterEvent::NodeJoined(node))
            .map_err(|e| IoError::runtime_error(format!("Failed to broadcast node join: {}", e)))?;

        // Start heartbeat
        self.start_heartbeat(node_id);
        Ok(())
    }

    fn start_health_monitor(&self) {
        let nodes = self.nodes.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.heartbeat_interval);
            loop {
                interval.tick().await;
                let mut nodes = nodes.write().await;
                
                // Check node health
                nodes.retain(|id, node| {
                    let is_healthy = node.last_heartbeat.elapsed() < config.node_timeout;
                    if !is_healthy {
                        let _ = event_tx.send(ClusterEvent::NodeLeft(id.clone()));
                    }
                    is_healthy
                });
            }
        });
    }

    fn start_leader_election(&self) {
        let nodes = self.nodes.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            loop {
                if Self::needs_leader_election(&nodes).await {
                    if let Some(new_leader) = Self::elect_leader(&nodes).await {
                        let _ = event_tx.send(ClusterEvent::LeaderElected(new_leader));
                    }
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    async fn needs_leader_election(nodes: &Arc<RwLock<HashMap<String, NodeInfo>>>) -> bool {
        let nodes = nodes.read().await;
        !nodes.values().any(|node| node.role == NodeRole::Leader)
    }

    async fn elect_leader(nodes: &Arc<RwLock<HashMap<String, NodeInfo>>>) -> Option<String> {
        let mut nodes = nodes.write().await;
        
        // Simple election: choose node with lowest ID as leader
        if let Some((leader_id, leader_node)) = nodes.iter_mut()
            .min_by_key(|(id, _)| *id)
        {
            leader_node.role = NodeRole::Leader;
            return Some(leader_id.clone());
        }
        None
    }

    pub async fn get_node_info(&self, node_id: &str) -> Option<NodeInfo> {
        self.nodes.read().await.get(node_id).cloned()
    }

    pub async fn update_metrics(&self, node_id: &str, metrics: NodeMetrics) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.metrics = metrics;
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Node {} not found", node_id)))
        }
    }
}

fn generate_node_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

impl Default for NodeMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            load_average: 0.0,
            active_tasks: 0,
        }
    }
}
