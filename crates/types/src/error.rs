use thiserror::Error;

/// Errors that can occur during EVM execution.
#[derive(Debug, Error)]
pub enum EvmError {
    /// Transaction validation failed before execution.
    #[error("transaction validation failed: {0}")]
    TransactionValidation(String),

    /// State access error (e.g., database read failure).
    #[error("state access error: {0}")]
    StateAccess(String),

    /// EVM internal error.
    #[error("evm internal error: {0}")]
    Internal(String),

    /// Precompile execution failed.
    #[error("precompile error at {address}: {reason}")]
    Precompile {
        /// Address of the precompile that failed.
        address: String,
        /// Reason for the failure.
        reason: String,
    },

    /// Block environment is invalid.
    #[error("invalid block environment: {0}")]
    InvalidBlockEnv(String),

    /// Reserve balance error (MIP-4).
    #[error("reserve balance error: {0}")]
    ReserveBalance(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = EvmError::TransactionValidation("nonce too low".to_string());
        assert_eq!(
            err.to_string(),
            "transaction validation failed: nonce too low"
        );
    }

    #[test]
    fn test_state_access_error() {
        let err = EvmError::StateAccess("database connection lost".to_string());
        assert!(err.to_string().contains("database connection lost"));
    }

    #[test]
    fn test_precompile_error() {
        let err = EvmError::Precompile {
            address: "0x01".to_string(),
            reason: "invalid input length".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "precompile error at 0x01: invalid input length"
        );
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EvmError>();
    }
}
