use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tracks system-level metrics for the node
pub struct SystemMetrics {
    // Memory metrics
    total_memory: Mutex<u64>,
    used_memory: Mutex<u64>,
    memory_utilization_pct: Mutex<f64>,
    total_swap: Mutex<u64>,
    used_swap: Mutex<u64>,
    
    // CPU metrics
    cpu_usage_pct: Mutex<f64>,
    cpu_count: Mutex<u64>,
    
    // Process metrics
    process_memory: Mutex<u64>,
    process_cpu_pct: Mutex<f64>,
    process_uptime: Mutex<u64>,
    process_read_bytes: Mutex<u64>,
    process_written_bytes: Mutex<u64>,
    
    // Disk metrics
    disk_metrics: Mutex<HashMap<String, DiskMetric>>,
    
    // Network metrics
    network_metrics: Mutex<HashMap<String, NetworkMetric>>,
    
    // Uptime metrics
    node_start_time: Instant,
    
    // Collection metrics
    last_collection_duration: Mutex<f64>,
}

/// Disk metrics for a specific disk
pub struct DiskMetric {
    pub total_space: u64,
    pub available_space: u64,
    pub usage_pct: f64,
    pub disk_type: String,
    pub last_updated: Instant,
}

/// Network metrics for a specific interface
pub struct NetworkMetric {
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
    pub received_packets: u64,
    pub transmitted_packets: u64,
    pub receive_errors: u64,
    pub transmit_errors: u64,
    pub bytes_per_sec_in: f64,
    pub bytes_per_sec_out: f64,
    pub last_updated: Instant,
    pub previous_received: u64,
    pub previous_transmitted: u64,
}

impl SystemMetrics {
    /// Create a new SystemMetrics structure
    pub fn new() -> Self {
        Self {
            total_memory: Mutex::new(0),
            used_memory: Mutex::new(0),
            memory_utilization_pct: Mutex::new(0.0),
            total_swap: Mutex::new(0),
            used_swap: Mutex::new(0),
            
            cpu_usage_pct: Mutex::new(0.0),
            cpu_count: Mutex::new(0),
            
            process_memory: Mutex::new(0),
            process_cpu_pct: Mutex::new(0.0),
            process_uptime: Mutex::new(0),
            process_read_bytes: Mutex::new(0),
            process_written_bytes: Mutex::new(0),
            
            disk_metrics: Mutex::new(HashMap::new()),
            network_metrics: Mutex::new(HashMap::new()),
            
            node_start_time: Instant::now(),
            last_collection_duration: Mutex::new(0.0),
        }
    }
    
    /// Record memory usage metrics
    pub fn record_memory_usage(&self, total: u64, used: u64, total_swap: u64, used_swap: u64) {
        let memory_pct = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        *self.total_memory.lock().unwrap() = total;
        *self.used_memory.lock().unwrap() = used;
        *self.memory_utilization_pct.lock().unwrap() = memory_pct;
        *self.total_swap.lock().unwrap() = total_swap;
        *self.used_swap.lock().unwrap() = used_swap;
    }
    
    /// Record CPU usage metrics
    pub fn record_cpu_usage(&self, usage_pct: f64, cpu_count: u64) {
        *self.cpu_usage_pct.lock().unwrap() = usage_pct;
        *self.cpu_count.lock().unwrap() = cpu_count;
    }
    
    /// Record process-specific metrics
    pub fn record_process_metrics(
        &self,
        memory: u64,
        cpu_pct: f64,
        uptime: u64,
        read_bytes: u64,
        written_bytes: u64,
    ) {
        *self.process_memory.lock().unwrap() = memory;
        *self.process_cpu_pct.lock().unwrap() = cpu_pct;
        *self.process_uptime.lock().unwrap() = uptime;
        *self.process_read_bytes.lock().unwrap() = read_bytes;
        *self.process_written_bytes.lock().unwrap() = written_bytes;
    }
    
    /// Record disk metrics for a specific disk
    pub fn record_disk_metrics(
        &self,
        disk_name: String,
        total_space: u64,
        available_space: u64,
        disk_type: String,
    ) {
        let used_space = total_space.saturating_sub(available_space);
        let usage_pct = if total_space > 0 {
            (used_space as f64 / total_space as f64) * 100.0
        } else {
            0.0
        };
        
        let mut disks = self.disk_metrics.lock().unwrap();
        disks.insert(
            disk_name,
            DiskMetric {
                total_space,
                available_space,
                usage_pct,
                disk_type,
                last_updated: Instant::now(),
            },
        );
    }
    
    /// Record network metrics for a specific interface
    pub fn record_network_metrics(
        &self,
        interface_name: String,
        received_bytes: u64,
        transmitted_bytes: u64,
        received_packets: u64,
        transmitted_packets: u64,
        receive_errors: u64,
        transmit_errors: u64,
    ) {
        let mut networks = self.network_metrics.lock().unwrap();
        let now = Instant::now();
        
        // Calculate rate if we have previous values
        let (bytes_per_sec_in, bytes_per_sec_out, previous_received, previous_transmitted) = 
            if let Some(prev) = networks.get(&interface_name) {
                let elapsed = now.duration_since(prev.last_updated).as_secs_f64();
                if elapsed > 0.0 {
                    let bps_in = (received_bytes.saturating_sub(prev.previous_received) as f64) / elapsed;
                    let bps_out = (transmitted_bytes.saturating_sub(prev.previous_transmitted) as f64) / elapsed;
                    (bps_in, bps_out, received_bytes, transmitted_bytes)
                } else {
                    (0.0, 0.0, prev.previous_received, prev.previous_transmitted)
                }
            } else {
                (0.0, 0.0, received_bytes, transmitted_bytes)
            };
        
        networks.insert(
            interface_name,
            NetworkMetric {
                received_bytes,
                transmitted_bytes,
                received_packets,
                transmitted_packets,
                receive_errors,
                transmit_errors,
                bytes_per_sec_in,
                bytes_per_sec_out,
                last_updated: now,
                previous_received,
                previous_transmitted,
            },
        );
    }
    
    /// Record the time taken to collect metrics
    pub fn record_metrics_collection_time(&self, duration_secs: f64) {
        *self.last_collection_duration.lock().unwrap() = duration_secs;
    }
    
    /// Get the node uptime in seconds
    pub fn node_uptime(&self) -> u64 {
        self.node_start_time.elapsed().as_secs()
    }
    
    /// Get memory usage information
    pub fn memory_usage(&self) -> (u64, u64, f64) {
        (
            *self.total_memory.lock().unwrap(),
            *self.used_memory.lock().unwrap(),
            *self.memory_utilization_pct.lock().unwrap(),
        )
    }
    
    /// Get CPU usage information
    pub fn cpu_usage(&self) -> (f64, u64) {
        (
            *self.cpu_usage_pct.lock().unwrap(),
            *self.cpu_count.lock().unwrap(),
        )
    }
    
    /// Get process metrics
    pub fn process_metrics(&self) -> (u64, f64, u64, u64, u64) {
        (
            *self.process_memory.lock().unwrap(),
            *self.process_cpu_pct.lock().unwrap(),
            *self.process_uptime.lock().unwrap(),
            *self.process_read_bytes.lock().unwrap(),
            *self.process_written_bytes.lock().unwrap(),
        )
    }
    
    /// Get collection of disk metrics
    pub fn disk_metrics(&self) -> HashMap<String, DiskMetric> {
        self.disk_metrics.lock().unwrap().clone()
    }
    
    /// Get collection of network metrics
    pub fn network_metrics(&self) -> HashMap<String, NetworkMetric> {
        self.network_metrics.lock().unwrap().clone()
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DiskMetric {
    fn clone(&self) -> Self {
        Self {
            total_space: self.total_space,
            available_space: self.available_space,
            usage_pct: self.usage_pct,
            disk_type: self.disk_type.clone(),
            last_updated: self.last_updated,
        }
    }
}

impl Clone for NetworkMetric {
    fn clone(&self) -> Self {
        Self {
            received_bytes: self.received_bytes,
            transmitted_bytes: self.transmitted_bytes,
            received_packets: self.received_packets,
            transmitted_packets: self.transmitted_packets,
            receive_errors: self.receive_errors,
            transmit_errors: self.transmit_errors,
            bytes_per_sec_in: self.bytes_per_sec_in,
            bytes_per_sec_out: self.bytes_per_sec_out,
            last_updated: self.last_updated,
            previous_received: self.previous_received,
            previous_transmitted: self.previous_transmitted,
        }
    }
} 