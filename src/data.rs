use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuditEvent {
    pub context: String,
    pub origin: String,
    pub start: u64,
    pub end: u64,
    pub events: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub spans: Vec<AuditEvent>,
}

impl AuditEvent {
    pub fn name(&self) -> String {
        self.events
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    }

    pub fn format_details(&self) -> String {
        let name = self.name();
        let mut details = Vec::new();

        if name.starts_with("tls::handshake_") {
            if let Some(version) = self.events.get("tls::protocol_version") {
                if let Some(v) = version.as_u64() {
                    match v {
                        772 => details.push("TLS 1.3".to_string()),
                        771 => details.push("TLS 1.2".to_string()),
                        _ => details.push(format!("version {}", v)),
                    }
                }
            }
            if let Some(cs) = self.events.get("tls::ciphersuite") {
                details.push(format!("ciphersuite {}", cs));
            }
        } else if name == "tls::sign" || name == "tls::verify" {
            if let Some(sig) = self.events.get("tls::signature_algorithm") {
                if let Some(s) = sig.as_u64() {
                    let sig_name = match s {
                        1027 => "ecdsa_secp256r1_sha256",
                        2052 => "rsa_pss_rsae_sha256",
                        _ => "unknown",
                    };
                    details.push(sig_name.to_string());
                }
            }
        } else if name == "tls::key_exchange" {
            if let Some(group) = self.events.get("tls::group") {
                if let Some(g) = group.as_u64() {
                    let group_name = match g {
                        23 => "secp256r1",
                        4588 => "X25519MLKEM768",
                        _ => "unknown",
                    };
                    details.push(group_name.to_string());
                }
            }
        } else if name.starts_with("pk::") {
            if let Some(algo) = self.events.get("pk::algorithm") {
                if let Some(a) = algo.as_str() {
                    details.push(a.to_string());
                }
            }
            if let Some(bits) = self.events.get("pk::bits") {
                details.push(format!("{} bits", bits));
            }
        }

        if details.is_empty() {
            name
        } else {
            format!("{} [{}]", name, details.join(", "))
        }
    }
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub value: usize,
    pub children: Vec<TreeNode>,
}

impl AuditEvent {
    pub fn get_time_range(events: &[AuditEvent]) -> Option<(u64, u64)> {
        if events.is_empty() {
            return None;
        }

        let mut min_start = u64::MAX;
        let mut max_end = u64::MIN;

        fn update_range(event: &AuditEvent, min_start: &mut u64, max_end: &mut u64) {
            *min_start = (*min_start).min(event.start);
            *max_end = (*max_end).max(event.end);

            for span in &event.spans {
                update_range(span, min_start, max_end);
            }
        }

        for event in events {
            update_range(event, &mut min_start, &mut max_end);
        }

        if min_start == u64::MAX || max_end == u64::MIN {
            None
        } else {
            Some((min_start, max_end))
        }
    }
}

impl TreeNode {
    pub fn from_events(events: &[AuditEvent]) -> Self {
        let mut root = TreeNode {
            name: "all".to_string(),
            value: 0,
            children: Vec::new(),
        };

        // Group by context
        let mut context_map: HashMap<String, Vec<&AuditEvent>> = HashMap::new();
        for event in events {
            context_map
                .entry(event.context.clone())
                .or_default()
                .push(event);
        }

        for (context, context_events) in context_map {
            let mut context_node = TreeNode {
                name: context,
                value: context_events.len(),
                children: Vec::new(),
            };

            for event in context_events {
                let event_node = Self::build_event_tree(event);
                context_node.children.push(event_node);
            }

            root.children.push(context_node);
        }

        root.update_values();
        root
    }

    fn build_event_tree(event: &AuditEvent) -> Self {
        let mut node = TreeNode {
            name: event.format_details(),
            value: 1,
            children: Vec::new(),
        };

        for span in &event.spans {
            node.children.push(Self::build_event_tree(span));
        }

        node
    }

    fn update_values(&mut self) -> usize {
        if self.children.is_empty() {
            return self.value;
        }

        let mut total = 0;
        for child in &mut self.children {
            total += child.update_values();
        }

        self.value = total;
        total
    }

    pub fn extract_algorithm_stats(&self, stats: &mut HashMap<String, usize>) {
        // Check if this node represents a pk:: operation with algorithm info
        if self.name.starts_with("pk::") {
            // Extract algorithm from the name (it's in brackets)
            if let Some(start) = self.name.find('[') {
                if let Some(end) = self.name.find(']') {
                    let details = &self.name[start + 1..end];
                    // Look for algorithm name (it's the first part before comma or the whole thing)
                    for part in details.split(',') {
                        let trimmed = part.trim();
                        // Skip "bits" entries
                        if !trimmed.ends_with("bits") && !trimmed.is_empty() {
                            *stats.entry(trimmed.to_string()).or_insert(0) += self.value;
                            break; // Only count the algorithm name, not other details
                        }
                    }
                }
            }
        }

        // Recursively process children
        for child in &self.children {
            child.extract_algorithm_stats(stats);
        }
    }
}
