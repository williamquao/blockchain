use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ProofOfHistory {
    current_hash: Vec<u8>,
    count: u64,
    last_timestamp: u64,
}

impl ProofOfHistory {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        ProofOfHistory {
            current_hash: vec![0; 32],
            count: 0,
            last_timestamp: now,
        }
    }

    // Record a new event in the PoH sequence
    pub fn record_event(&mut self, data: &[u8]) -> (Vec<u8>, u64) {
        let mut hasher = Sha256::new();
        hasher.update(&self.current_hash);
        hasher.update(data);
        
        self.current_hash = hasher.finalize().to_vec();
        self.count += 1;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_timestamp = now;
        
        (self.current_hash.clone(), self.count)
    }

    // Verify that events occurred in the correct sequence
    pub fn verify_sequence(&self, previous_hash: &[u8], data: &[u8], expected_hash: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(previous_hash);
        hasher.update(data);
        
        let calculated_hash = hasher.finalize();
        calculated_hash.as_slice() == expected_hash
    }

    // Get the current tick count
    pub fn get_count(&self) -> u64 {
        self.count
    }

    // Get the current hash
    pub fn get_current_hash(&self) -> &Vec<u8> {
        &self.current_hash
    }

    // Get transactions per second
    pub fn get_tps(&self, start_count: u64, start_time: u64) -> f64 {
        let count_diff = self.count - start_count;
        let time_diff = self.last_timestamp - start_time;
        
        if time_diff == 0 {
            return 0.0;
        }
        
        count_diff as f64 / time_diff as f64
    }
} 