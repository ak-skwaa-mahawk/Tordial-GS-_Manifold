use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::{Arc, RwLock};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "record_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SovereignPayload {
    LinguistHandshakeNftCertificate {
        title: String,
        flameholder: String,
        issuer: String,
        historical_sha256: String,
        acknowledged_frequencies: Vec<f64>,
        root_languages: Vec<String>,
        tempo_key: String,
        device_node: String,
    },
    DigitalExecutorDeclaration {
        target_file: String,
        archive_source: String,
        archive_sha256: String,
        decryption_key_hint: String,
        unencrypted_zip_sha256: String,
        status: String,
    },
    RuntimeTelemetry {
        session_key_verification: String,
        telemetry_statement: String,
        verification_timestamp: f64,
    },
    GenesisAnchor {
        info: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MeshBlock {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: f64,
    pub data: SovereignPayload,
    pub nonce: u64,
    pub hash: String,
}

impl MeshBlock {
    pub fn calculate_hash(&self) -> String {
        // Enforce rigid string representation to eliminate whitespace variance across runtimes
        let serialized = serde_json::to_string(&self).unwrap_or_default();
        
        // Extract a deterministic subset excluding the hash signature itself for calculation
        let canonical_json = serde_json::json!({
            "index": self.index,
            "previous_hash": self.previous_hash,
            "timestamp": self.timestamp,
            "data": self.data,
            "nonce": self.nonce
        });

        let mut hasher = Sha256::new();
        hasher.update(canonical_json.to_string().as_bytes());
        hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

pub struct SubstrateMeshBus {
    pub ledger_file: String,
    pub difficulty: usize,
    pub chain: Arc<RwLock<Vec<MeshBlock>>>,
}

impl SubstrateMeshBus {
    pub fn new(ledger_file: &str, difficulty: usize) -> Self {
        let bus = SubstrateMeshBus {
            ledger_file: ledger_file.to_string(),
            difficulty,
            chain: Arc::new(RwLock::new(Vec::new())),
        };
        bus.sync_with_disk();
        bus
    }

    pub fn sync_with_disk(&self) {
        let mut chain_guard = self.chain.write().unwrap();
        if let Ok(mut file) = File::open(&self.ledger_file) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(parsed_chain) = serde_json::from_str::<Vec<MeshBlock>>(&contents) {
                    *chain_guard = parsed_chain;
                    return;
                }
            }
        }
        
        // Fallback: Initialize with Genesis if file is absent or broken
        let genesis_payload = SovereignPayload::GenesisAnchor {
            info: "Genesis Anchor — Tordial GS Manifold Sovereignty Established".to_string(),
        };
        
        let mut genesis = MeshBlock {
            index: 0,
            previous_hash: "0".repeat(64),
            timestamp: 1782455959.391051, // Preserving chronological baseline anchor
            data: genesis_payload,
            nonce: 0,
            hash: String::new(),
        };
        
        self.mine_block(&mut genesis);
        *chain_guard = vec![genesis];
        drop(chain_guard);
        self.flush_to_disk();
    }

    fn mine_block(&self, block: &mut MeshBlock) {
        let prefix = "0".repeat(self.difficulty);
        block.hash = block.calculate_hash();
        while !block.hash.starts_with(&prefix) {
            block.nonce += 1;
            block.hash = block.calculate_hash();
        }
    }

    pub fn append_record(&self, data: SovereignPayload) -> String {
        let mut chain_guard = self.chain.write().unwrap();
        let latest = chain_guard.last().unwrap();
        
        let mut new_block = MeshBlock {
            index: latest.index + 1,
            previous_hash: latest.hash.clone(),
            timestamp: chrono::Utc::now().timestamp_millis() as f64 / 1000.0,
            data,
            nonce: 0,
            hash: String::new(),
        };

        self.mine_block(&mut new_block);
        let commit_hash = new_block.hash.clone();
        chain_guard.append(&mut vec![new_block]);
        
        drop(chain_guard);
        self.flush_to_disk();
        commit_hash
    }

    pub fn flush_to_disk(&self) {
        let chain_guard = self.chain.read().unwrap();
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.ledger_file)
        {
            let json_out = serde_json::to_string_pretty(&*chain_guard).unwrap_or_default();
            let _ = file.write_all(json_out.as_bytes());
        }
    }

    pub fn verify_chain_integrity(&self) -> bool {
        let chain_guard = self.chain.read().unwrap();
        let prefix = "0".repeat(self.difficulty);

        for i in 0..chain_guard.len() {
            let current = &chain_guard[i];
            if current.hash != current.calculate_hash() || !current.hash.starts_with(&prefix) {
                return false;
            }
            if i > 0 {
                let previous = &chain_guard[i - 1];
                if current.previous_hash != previous.hash {
                    return false;
                }
            }
        }
        true
    }
}
