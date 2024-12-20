use btclib::types::block::Block;
use btclib::types::transaction::{Transaction, TransactionInput, TransactionOutput};
use super::worker::MiningWorker;
use super::template::{BlockTemplate, MempoolInterface, BLOCK_MAX_SIZE};
use crate::difficulty::DifficultyAdjuster;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tracing::{info, error};
use async_trait::async_trait;


pub struct Miner {
    workers: Vec<MiningWorker>,
    difficulty_adjuster: DifficultyAdjuster,
    stop_signal: Arc<AtomicBool>,
    block_sender: mpsc::Sender<Block>,
    num_threads: usize,
    mempool: Arc<dyn MempoolInterface + Send + Sync>,
    reward_address: Vec<u8>,
}

impl Miner {
    pub fn new(
        num_threads: usize,
        initial_target: u32,
        mempool: Arc<dyn MempoolInterface + Send + Sync>,
        reward_address: Vec<u8>,
    ) -> (Self, mpsc::Receiver<Block>) {
        let (tx, rx) = mpsc::channel(100);
        let stop_signal = Arc::new(AtomicBool::new(false));

        let mut workers = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            workers.push(MiningWorker::new(
                Arc::clone(&stop_signal),
                tx.clone(),
                initial_target,
                i,
                Arc::clone(&mempool),
            ));
        }

        (Self {
            workers,
            difficulty_adjuster: DifficultyAdjuster::new(initial_target),
            stop_signal,
            block_sender: tx,
            num_threads,
            mempool,
            reward_address,
        }, rx)
    }

    pub async fn start_mining(
        &mut self,
        version: u32,
        prev_block_hash: [u8; 32],
        current_height: u64,
    ) -> Result<(), String> {
        info!("Starting mining with {} workers", self.num_threads);

        let mut handles = Vec::new();
        
        // Start all mining workers
        for worker in &self.workers {
            let reward_address = self.reward_address.clone();
            handles.push(tokio::spawn(async move {
                worker.mine_block(
                    version,
                    prev_block_hash,
                    reward_address,
                ).await
            }));
        }

        // Wait for any worker to find a block
        for handle in handles {
            if let Err(e) = handle.await {
                error!("Mining task error: {}", e);
            }
        }

        Ok(())
    }

    pub fn stop_mining(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
        info!("Mining stopped");
    }

    pub fn adjust_difficulty(
        &mut self,
        current_height: u64,
        current_time: u64,
        blocks_since_adjustment: u64,
    ) {
        let new_target = self.difficulty_adjuster.adjust_difficulty(
            current_height,
            current_time,
            blocks_since_adjustment,
        );

        info!("Adjusting mining difficulty. New target: {:#x}", new_target);
        
        // Update all workers with new target
        for (i, worker) in self.workers.iter_mut().enumerate() {
            *worker = MiningWorker::new(
                Arc::clone(&self.stop_signal),
                self.block_sender.clone(),
                new_target,
                i,
                Arc::clone(&self.mempool),
            );
        }
    }

    pub fn get_current_target(&self) -> u32 {
        self.difficulty_adjuster.get_current_target()
    }

    pub fn set_reward_address(&mut self, address: Vec<u8>) {
        self.reward_address = address;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockMempool;
    
    #[async_trait]
    impl MempoolInterface for MockMempool {
        async fn get_transactions(&self, _max_size: usize) -> Vec<Transaction> {
            Vec::new()
        }
    }

    #[tokio::test]
    async fn test_miner_creation() {
        let mempool = Arc::new(MockMempool);
        let reward_address = vec![1, 2, 3, 4];
        let (miner, _rx) = Miner::new(4, 0x1d00ffff, mempool, reward_address);
        assert_eq!(miner.num_threads, 4);
        assert_eq!(miner.get_current_target(), 0x1d00ffff);
    }

    #[tokio::test]
    async fn test_mining_start_stop() {
        let mempool = Arc::new(MockMempool);
        let reward_address = vec![1, 2, 3, 4];
        let (miner, mut rx) = Miner::new(1, u32::MAX, mempool, reward_address); // Use maximum target for quick mining
        
        // Start mining in a separate task
        let mut miner = miner;
        let mining_handle = tokio::spawn(async move {
            miner.start_mining(1, [0u8; 32], 0).await.unwrap();
        });

        // Wait for a block or timeout
        tokio::select! {
            Some(block) = rx.recv() => {
                assert!(block.validate());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                miner.stop_mining();
            }
        }

        mining_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_difficulty_adjustment() {
        let mempool = Arc::new(MockMempool);
        let reward_address = vec![1, 2, 3, 4];
        let (mut miner, _rx) = Miner::new(1, 0x1d00ffff, mempool, reward_address);
        let initial_target = miner.get_current_target();

        miner.adjust_difficulty(2016, 60 * 1008, 2016); // Half the expected time
        assert!(miner.get_current_target() < initial_target);
    }
}