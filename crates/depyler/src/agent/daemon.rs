//! Background Daemon for Depyler Agent Mode
//!
//! Manages the lifecycle of the Depyler background agent service with graceful
//! startup, shutdown, and continuous Python-to-Rust transpilation capabilities.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::signal;
use tokio::sync::{RwLock, mpsc};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use super::mcp_server::DepylerMcpServer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub port: u16,
    pub debug: bool,
    pub auto_transpile: bool,
    pub verification_level: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            debug: false,
            auto_transpile: true,
            verification_level: "basic".to_string(),
        }
    }
}
use super::transpilation_monitor::{TranspilationEvent, TranspilationMonitorConfig, TranspilationMonitorEngine};

pub struct AgentDaemon {
    config: DaemonConfig,
    mcp_server: Option<DepylerMcpServer>,
    transpilation_monitor: Option<TranspilationMonitorEngine>,
    state: Arc<RwLock<DaemonState>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub agent: AgentConfig,
    pub transpilation_monitor: TranspilationMonitorConfig,
    pub daemon: DaemonSettings,
    pub mcp_port: u16,
    pub debug: bool,
}

impl DaemonConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            transpilation_monitor: TranspilationMonitorConfig::default(),
            daemon: DaemonSettings::default(),
            mcp_port: 3000,
            debug: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSettings {
    pub pid_file: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub working_directory: PathBuf,
    pub health_check_interval: Duration,
    pub max_memory_mb: u64,
    pub auto_restart: bool,
    pub shutdown_timeout: Duration,
    pub auto_transpile: bool,
    pub verification_level: VerificationLevel,
}

impl Default for DaemonSettings {
    fn default() -> Self {
        Self {
            pid_file: None,
            log_file: None,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            health_check_interval: Duration::from_secs(30),
            max_memory_mb: 1000, // More memory for transpilation
            auto_restart: true,
            shutdown_timeout: Duration::from_secs(10),
            auto_transpile: true,
            verification_level: VerificationLevel::Basic,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum VerificationLevel {
    None,
    #[default]
    Basic,
    Full,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonState {
    pub status: DaemonStatus,
    pub start_time: SystemTime,
    pub last_health_check: SystemTime,
    pub memory_usage_mb: u64,
    pub monitored_projects: usize,
    pub total_transpilations: u64,
    pub successful_transpilations: u64,
    pub failed_transpilations: u64,
    pub last_error: Option<String>,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            status: DaemonStatus::Starting,
            start_time: SystemTime::now(),
            last_health_check: SystemTime::now(),
            memory_usage_mb: 0,
            monitored_projects: 0,
            total_transpilations: 0,
            successful_transpilations: 0,
            failed_transpilations: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DaemonStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
    Restarting,
}

impl AgentDaemon {
    pub fn new(config: DaemonConfig) -> Self {
        let state = Arc::new(RwLock::new(DaemonState::default()));

        Self {
            config,
            mcp_server: None,
            transpilation_monitor: None,
            state,
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Depyler Agent Daemon...");

        {
            let mut state = self.state.write().await;
            state.status = DaemonStatus::Starting;
            state.start_time = SystemTime::now();
        }

        if let Some(pid_file) = &self.config.daemon.pid_file {
            let pid = std::process::id();
            std::fs::write(pid_file, pid.to_string())
                .map_err(|e| anyhow::anyhow!("Failed to write PID file: {}", e))?;
            info!("PID {} written to {:?}", pid, pid_file);
        }

        std::env::set_current_dir(&self.config.daemon.working_directory)
            .map_err(|e| anyhow::anyhow!("Failed to change working directory: {}", e))?;

        let mcp_server = DepylerMcpServer::new();
        self.mcp_server = Some(mcp_server);

        let transpilation_monitor = TranspilationMonitorEngine::new(self.config.transpilation_monitor.clone()).await?;
        self.transpilation_monitor = Some(transpilation_monitor);

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        {
            let mut state = self.state.write().await;
            state.status = DaemonStatus::Running;
        }

        info!("Depyler Agent Daemon started successfully");

        self.run_main_loop(shutdown_rx).await
    }

    async fn run_main_loop(&mut self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut health_check_interval = interval(self.config.daemon.health_check_interval);
        let mut transpilation_events = self.transpilation_monitor.as_mut().map(|tm| tm.get_event_receiver());

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }

                _ = signal::ctrl_c() => {
                    info!("Received Ctrl+C signal");
                    break;
                }

                _ = health_check_interval.tick() => {
                    if let Err(e) = self.perform_health_check().await {
                        error!("Health check failed: {}", e);

                        let mut state = self.state.write().await;
                        state.last_error = Some(e.to_string());

                        if self.config.daemon.auto_restart {
                        warn!("Auto-restart enabled, restarting daemon...");
                        state.status = DaemonStatus::Restarting;
                    }
                }
            }                event = async {
                    match transpilation_events.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => None
                    }
                } => {
                    if let Some(event) = event {
                        if let Err(e) = self.handle_transpilation_event(event).await {
                            error!("Failed to handle transpilation event: {}", e);
                        }
                    }
                }
            }
        }

        self.shutdown().await
    }

    async fn perform_health_check(&self) -> Result<()> {
        debug!("Performing health check...");

        let memory_usage = self.get_memory_usage().await?;
        if memory_usage > self.config.daemon.max_memory_mb {
            bail!(
                "Memory usage ({} MB) exceeds limit ({} MB)",
                memory_usage,
                self.config.daemon.max_memory_mb
            );
        }

        {
            let mut state = self.state.write().await;
            state.last_health_check = SystemTime::now();
            state.memory_usage_mb = memory_usage;
        }

        debug!("Health check passed (memory: {} MB)", memory_usage);
        Ok(())
    }

    async fn get_memory_usage(&self) -> Result<u64> {
        #[cfg(unix)]
        {
            use std::fs;
            let status = fs::read_to_string("/proc/self/status")?;
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let kb: u64 = parts[1].parse().unwrap_or(0);
                        return Ok(kb / 1024);
                    }
                }
            }
        }

        Ok(100)
    }

    async fn handle_transpilation_event(&self, event: TranspilationEvent) -> Result<()> {
        info!("Handling transpilation event: {:?}", event);

        match event {
            TranspilationEvent::FileChanged { path, .. } => {
                if self.config.daemon.auto_transpile {
                    match self.transpile_file(&path).await {
                        Ok(_) => {
                            let mut state = self.state.write().await;
                            state.total_transpilations += 1;
                            state.successful_transpilations += 1;
                        }
                        Err(e) => {
                            error!("Failed to transpile {}: {}", path.display(), e);
                            let mut state = self.state.write().await;
                            state.total_transpilations += 1;
                            state.failed_transpilations += 1;
                            state.last_error = Some(e.to_string());
                        }
                    }
                }
            }
            TranspilationEvent::ProjectAdded { .. } => {
                let mut state = self.state.write().await;
                state.monitored_projects += 1;
                info!("Now monitoring {} projects", state.monitored_projects);
            }
            TranspilationEvent::ProjectRemoved { project_id: _ } => {
                let mut state = self.state.write().await;
                state.monitored_projects = state.monitored_projects.saturating_sub(1);
                info!("Now monitoring {} projects", state.monitored_projects);
            }
            TranspilationEvent::TranspilationSucceeded { project_id, .. } => {
                debug!("Transpilation succeeded for project '{}'", project_id);
                let mut state = self.state.write().await;
                state.successful_transpilations += 1;
            }
            TranspilationEvent::TranspilationFailed { project_id, error, .. } => {
                warn!("Transpilation failed for project '{}': {}", project_id, error);
                let mut state = self.state.write().await;
                state.failed_transpilations += 1;
                state.last_error = Some(error);
            }
            TranspilationEvent::StatusUpdate { .. } => {
                debug!("Received transpilation status update");
            }
        }

        Ok(())
    }

    async fn transpile_file(&self, path: &std::path::Path) -> Result<()> {
        use depyler_core::DepylerPipeline;

        let source = std::fs::read_to_string(path)?;
        let pipeline = DepylerPipeline::new();
        let result = pipeline.transpile(&source)?;
        let output_path = path.with_extension("rs");

        std::fs::write(&output_path, result)?;

        info!("Transpiled {} -> {}", path.display(), output_path.display());

        if self.config.daemon.verification_level != VerificationLevel::None {
            self.verify_transpiled_code(&output_path).await?;
        }

        Ok(())
    }

    async fn verify_transpiled_code(&self, rust_path: &std::path::Path) -> Result<()> {
        match self.config.daemon.verification_level {
            VerificationLevel::None => Ok(()),
            VerificationLevel::Basic => {
                let output = std::process::Command::new("rustc")
                    .arg("--parse-only")
                    .arg(rust_path)
                    .output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    bail!("Rust syntax check failed: {}", stderr);
                }

                Ok(())
            }
            VerificationLevel::Full => {
                let output = std::process::Command::new("rustc")
                    .arg("--check")
                    .arg(rust_path)
                    .output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    bail!("Rust compilation check failed: {}", stderr);
                }

                Ok(())
            }
            VerificationLevel::Strict => {
                let mut cmd = std::process::Command::new("cargo");
                cmd.args(["clippy", "--", "-D", "warnings"])
                    .current_dir(rust_path.parent().unwrap_or_else(|| std::path::Path::new(".")));

                let output = cmd.output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    bail!("Rust strict verification failed: {}", stderr);
                }

                Ok(())
            }
        }
    }

    pub async fn get_state(&self) -> DaemonState {
        self.state.read().await.clone()
    }

    pub async fn request_shutdown(&self) -> Result<()> {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            shutdown_tx.send(()).await?;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Depyler Agent Daemon in foreground mode");
        self.start().await
    }

    pub async fn start_daemon(&mut self) -> Result<()> {
        info!("Starting Depyler Agent Daemon in background mode");
        self.start().await
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down daemon...");

        if let Some(mcp_server) = self.mcp_server.take() {
            if let Err(e) = mcp_server.shutdown().await {
                error!("Failed to shutdown MCP server: {}", e);
            }
        }

        if let Some(mut monitor) = self.transpilation_monitor.take() {
            if let Err(e) = monitor.shutdown().await {
                error!("Failed to shutdown transpilation monitor: {}", e);
            }
        }

        info!("Depyler Agent Daemon shut down successfully");
        Ok(())
    }

    pub fn stop_daemon() -> Result<()> {
        let pid_file = std::env::temp_dir().join("depyler_agent.pid");
        if pid_file.exists() {
            let pid_str = std::fs::read_to_string(&pid_file)?;
            let pid = pid_str.trim().parse::<i32>()?;

            #[cfg(unix)]
            {
                use std::process::Command;
                let _ = Command::new("kill").arg(pid.to_string()).output();
            }

            std::fs::remove_file(&pid_file)?;
            info!("Daemon stopped (PID: {})", pid);
        } else {
            info!("No daemon PID file found");
        }
        Ok(())
    }

    pub fn daemon_status() -> Result<Option<i32>> {
        let pid_file = std::env::temp_dir().join("depyler_agent.pid");
        if pid_file.exists() {
            let pid_str = std::fs::read_to_string(&pid_file)?;
            let pid = pid_str.trim().parse::<i32>()?;

            #[cfg(unix)]
            {
                use std::process::Command;
                let output = Command::new("ps").args(["-p", &pid.to_string()]).output()?;

                if output.status.success() {
                    Ok(Some(pid))
                } else {
                    let _ = std::fs::remove_file(&pid_file);
                    Ok(None)
                }
            }

            #[cfg(not(unix))]
            Ok(Some(pid))
        } else {
            Ok(None)
        }
    }

    pub fn show_logs(lines: usize) -> Result<()> {
        let log_file = std::env::temp_dir().join("depyler_agent.log");
        if log_file.exists() {
            let content = std::fs::read_to_string(&log_file)?;
            let lines_vec: Vec<&str> = content.lines().collect();
            let start = lines_vec.len().saturating_sub(lines);

            for line in &lines_vec[start..] {
                println!("{}", line);
            }
        } else {
            println!("No log file found");
        }
        Ok(())
    }

    pub fn tail_logs() -> Result<()> {
        println!("Log following not yet implemented. Use 'depyler agent logs' to view recent logs.");
        Ok(())
    }
}
