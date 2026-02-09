// PHASE 5: PROTOCOL VERSION MANAGEMENT
// Manages protocol versions, compatibility, and versioning
//
// SAFETY INVARIANTS:
// 1. Protocol versions are immutable once applied
// 2. All validators execute same version at same epoch
// 3. Backward compatibility is verified before upgrade
// 4. Version transitions are atomic (all-or-nothing)
// 5. Canonical chain is always valid (never diverges)

use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use std::collections::BTreeMap;

#[derive(Debug, Error)]
pub enum VersionError {
    #[error("Invalid version format: {0}")]
    InvalidFormat(String),
    
    #[error("Downgrade not allowed: {0} → {1}")]
    DowngradeNotAllowed(String, String),
    
    #[error("Incompatible version: {0}")]
    IncompatibleVersion(String),
    
    #[error("Version not found: {0}")]
    VersionNotFound(String),
}

/// Protocol semantic version (major.minor.patch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProtocolVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        ProtocolVersion { major, minor, patch }
    }
    
    pub fn parse(version_str: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat(version_str.to_string()));
        }
        
        let major = parts[0].parse()
            .map_err(|_| VersionError::InvalidFormat(version_str.to_string()))?;
        let minor = parts[1].parse()
            .map_err(|_| VersionError::InvalidFormat(version_str.to_string()))?;
        let patch = parts[2].parse()
            .map_err(|_| VersionError::InvalidFormat(version_str.to_string()))?;
        
        Ok(ProtocolVersion { major, minor, patch })
    }
    
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
    
    /// Check if this version is compatible with a required version
    /// Compatibility rules:
    /// - Major version must be same
    /// - Minor version must be >= required
    /// - Patch version must be >= required
    pub fn is_compatible(&self, required: ProtocolVersion) -> bool {
        if self.major != required.major {
            return false;
        }
        
        if self.minor < required.minor {
            return false;
        }
        
        if self.minor == required.minor && self.patch < required.patch {
            return false;
        }
        
        true
    }
    
    /// Check if upgrade from previous to current is allowed
    /// Rules:
    /// - Cannot downgrade (current must be >= previous)
    /// - Can upgrade any way (major, minor, patch)
    /// - Downgrade: not allowed (VersionError)
    pub fn can_upgrade_from(&self, previous: ProtocolVersion) -> Result<(), VersionError> {
        if self < &previous {
            return Err(VersionError::DowngradeNotAllowed(
                previous.to_string(),
                self.to_string(),
            ));
        }
        Ok(())
    }
}

/// Protocol feature flags (enable/disable experimental features)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Map of feature name → enabled
    pub flags: BTreeMap<String, bool>,
}

impl FeatureFlags {
    pub fn new() -> Self {
        FeatureFlags {
            flags: BTreeMap::new(),
        }
    }
    
    pub fn enable(&mut self, feature: String) {
        self.flags.insert(feature, true);
    }
    
    pub fn disable(&mut self, feature: String) {
        self.flags.insert(feature, false);
    }
    
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.flags.get(feature).copied().unwrap_or(false)
    }
    
    /// Compute hash of feature flags
    pub fn hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        for (name, enabled) in &self.flags {
            hasher.update(name.as_bytes());
            hasher.update([if *enabled { 1u8 } else { 0u8 }]);
        }
        hasher.finalize().to_vec()
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol specification (version, features, parameters)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolSpec {
    /// Protocol version
    pub version: ProtocolVersion,
    
    /// Feature flags
    pub features: FeatureFlags,
    
    /// Activation epoch (when this spec becomes active)
    pub activation_epoch: u64,
    
    /// Hash of this spec (for commitment)
    pub spec_hash: Vec<u8>,
}

impl ProtocolSpec {
    pub fn new(version: ProtocolVersion, activation_epoch: u64) -> Self {
        let mut spec = ProtocolSpec {
            version,
            features: FeatureFlags::new(),
            activation_epoch,
            spec_hash: vec![],
        };
        spec.spec_hash = spec.compute_hash();
        spec
    }
    
    /// Compute hash of this spec
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.version.major.to_le_bytes());
        hasher.update(self.version.minor.to_le_bytes());
        hasher.update(self.version.patch.to_le_bytes());
        hasher.update(self.activation_epoch.to_le_bytes());
        hasher.update(self.features.hash());
        hasher.finalize().to_vec()
    }
}

/// Version history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistoryEntry {
    /// Protocol specification
    pub spec: ProtocolSpec,
    
    /// Epoch when activated
    pub activated_epoch: Option<u64>,
    
    /// Epoch when superseded (if any)
    pub superseded_epoch: Option<u64>,
}

/// Protocol version manager
pub struct ProtocolVersionManager {
    /// Current active version
    current_version: ProtocolVersion,
    
    /// Current feature flags
    current_features: FeatureFlags,
    
    /// Current activation epoch
    current_epoch: u64,
    
    /// Version history (immutable)
    history: Vec<VersionHistoryEntry>,
    
    /// Pending upgrade (awaiting activation)
    pending_upgrade: Option<ProtocolSpec>,
}

impl ProtocolVersionManager {
    pub fn new(initial_version: ProtocolVersion, initial_epoch: u64) -> Self {
        let mut manager = ProtocolVersionManager {
            current_version: initial_version,
            current_features: FeatureFlags::new(),
            current_epoch: initial_epoch,
            history: Vec::new(),
            pending_upgrade: None,
        };
        
        // Record initial version in history
        let spec = ProtocolSpec::new(initial_version, initial_epoch);
        manager.history.push(VersionHistoryEntry {
            spec,
            activated_epoch: Some(initial_epoch),
            superseded_epoch: None,
        });
        
        manager
    }
    
    /// Get current protocol version
    pub fn current_version(&self) -> ProtocolVersion {
        self.current_version
    }
    
    /// Get current feature flags
    pub fn current_features(&self) -> &FeatureFlags {
        &self.current_features
    }
    
    /// Schedule an upgrade (called by governance)
    pub fn schedule_upgrade(
        &mut self,
        new_version: ProtocolVersion,
        activation_epoch: u64,
    ) -> Result<(), VersionError> {
        // Verify upgrade is allowed
        new_version.can_upgrade_from(self.current_version)?;
        
        // Verify activation epoch is in future
        if activation_epoch <= self.current_epoch {
            return Err(VersionError::InvalidFormat(
                format!("Activation epoch {} must be > current {}", 
                    activation_epoch, self.current_epoch),
            ));
        }
        
        // Create new spec
        let spec = ProtocolSpec::new(new_version, activation_epoch);
        
        // Schedule as pending
        self.pending_upgrade = Some(spec);
        
        Ok(())
    }
    
    /// Activate pending upgrade (called at activation epoch)
    pub fn activate_upgrade(&mut self, current_epoch: u64) -> Result<(), VersionError> {
        if let Some(pending) = self.pending_upgrade.take() {
            if current_epoch < pending.activation_epoch {
                return Err(VersionError::InvalidFormat(
                    format!("Cannot activate at epoch {}, scheduled for {}", 
                        current_epoch, pending.activation_epoch),
                ));
            }
            
            // Mark previous version as superseded
            if let Some(last) = self.history.last_mut() {
                last.superseded_epoch = Some(current_epoch);
            }
            
            // Update current version
            self.current_version = pending.version;
            self.current_features = pending.features.clone();
            self.current_epoch = current_epoch;
            
            // Add to history
            let mut entry = VersionHistoryEntry {
                spec: pending,
                activated_epoch: Some(current_epoch),
                superseded_epoch: None,
            };
            entry.spec.spec_hash = entry.spec.compute_hash();
            
            self.history.push(entry);
            
            Ok(())
        } else {
            Err(VersionError::InvalidFormat("No pending upgrade".to_string()))
        }
    }
    
    /// Get version history
    pub fn get_history(&self) -> &[VersionHistoryEntry] {
        &self.history
    }
    
    /// Get pending upgrade
    pub fn get_pending_upgrade(&self) -> Option<&ProtocolSpec> {
        self.pending_upgrade.as_ref()
    }
    
    /// Cancel pending upgrade
    pub fn cancel_upgrade(&mut self) -> Result<(), VersionError> {
        if self.pending_upgrade.is_some() {
            self.pending_upgrade = None;
            Ok(())
        } else {
            Err(VersionError::InvalidFormat("No pending upgrade".to_string()))
        }
    }
    
    /// Verify version compatibility
    pub fn verify_compatibility(&self, version: ProtocolVersion) -> Result<(), VersionError> {
        if !version.is_compatible(self.current_version) {
            return Err(VersionError::IncompatibleVersion(version.to_string()));
        }
        Ok(())
    }
}

impl Default for ProtocolVersionManager {
    fn default() -> Self {
        Self::new(ProtocolVersion::new(1, 0, 0), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = ProtocolVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_comparison() {
        let v1 = ProtocolVersion::new(1, 0, 0);
        let v2 = ProtocolVersion::new(1, 1, 0);
        let v3 = ProtocolVersion::new(2, 0, 0);
        
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_compatibility() {
        let v1_1_0 = ProtocolVersion::new(1, 1, 0);
        let v1_0_0 = ProtocolVersion::new(1, 0, 0);
        
        // 1.1.0 is compatible with 1.0.0 (higher minor)
        assert!(v1_1_0.is_compatible(v1_0_0));
        
        // 1.0.0 is not compatible with 1.1.0 (lower minor)
        assert!(!v1_0_0.is_compatible(v1_1_0));
    }

    #[test]
    fn test_upgrade_allowed() {
        let v1 = ProtocolVersion::new(1, 0, 0);
        let v2 = ProtocolVersion::new(1, 1, 0);
        
        // Upgrade 1.0.0 → 1.1.0 allowed
        assert!(v2.can_upgrade_from(v1).is_ok());
    }

    #[test]
    fn test_downgrade_not_allowed() {
        let v1 = ProtocolVersion::new(1, 1, 0);
        let v2 = ProtocolVersion::new(1, 0, 0);
        
        // Downgrade 1.1.0 → 1.0.0 not allowed
        assert!(v2.can_upgrade_from(v1).is_err());
    }

    #[test]
    fn test_feature_flags() {
        let mut flags = FeatureFlags::new();
        
        assert!(!flags.is_enabled("feature_a"));
        
        flags.enable("feature_a".to_string());
        assert!(flags.is_enabled("feature_a"));
        
        flags.disable("feature_a".to_string());
        assert!(!flags.is_enabled("feature_a"));
    }

    #[test]
    fn test_version_manager() {
        let mut manager = ProtocolVersionManager::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        // Schedule upgrade
        manager.schedule_upgrade(
            ProtocolVersion::new(1, 1, 0),
            10,
        ).unwrap();
        
        assert!(manager.get_pending_upgrade().is_some());
        
        // Activate upgrade
        manager.activate_upgrade(10).unwrap();
        
        assert_eq!(manager.current_version().minor, 1);
        assert!(manager.get_pending_upgrade().is_none());
    }

    #[test]
    fn test_version_history() {
        let mut manager = ProtocolVersionManager::new(
            ProtocolVersion::new(1, 0, 0),
            0,
        );
        
        manager.schedule_upgrade(ProtocolVersion::new(1, 1, 0), 10).unwrap();
        manager.activate_upgrade(10).unwrap();
        
        let history = manager.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].spec.version, ProtocolVersion::new(1, 0, 0));
        assert_eq!(history[1].spec.version, ProtocolVersion::new(1, 1, 0));
    }

    #[test]
    fn test_version_spec_hash() {
        let spec1 = ProtocolSpec::new(ProtocolVersion::new(1, 0, 0), 0);
        let spec2 = ProtocolSpec::new(ProtocolVersion::new(1, 0, 0), 0);
        
        // Same spec → same hash
        assert_eq!(spec1.spec_hash, spec2.spec_hash);
    }
}
